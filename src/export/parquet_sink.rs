//! Parquet writer for locally-recorded CSI sessions.
//!
//! Rows are buffered and flushed as row groups; the file footer is written on close/drop.
//!
//! ## Schema
//! One superset schema covers every firmware version and chip, so consumers see a stable column
//! set. It is versioned: the file's key-value metadata carries `schema_version`
//! ([`SCHEMA_VERSION`]), plus `wire_version` (the [`crate::wire`] version this client decodes) and
//! `producer`. Columns a frame cannot fill are null.
//!
//! - **Provenance:** `host_rx_time` (client wall clock, UTC µs), `chip`, `stream_format`
//!   (`wire` / `legacy`), and from the wire envelope `wire_version`, `node_id`, `session_id`,
//!   `stream_seq`, `source`.
//! - **Time:** `timestamp_us` (device clock, 64-bit), `timestamp` (its low 32 bits, as 0.11 and
//!   the text formats print it) and `device_time` (wall clock, when a session announcement
//!   anchored the device clock).
//! - **Receive metadata:** `mac` (the transmitter: the header's `addr2`, else the stimulus'),
//!   `rssi`, `noise_floor`, `channel`, `sig_len`, `rx_state`, `sequence_number` (802.11 sequence
//!   number), `ppdu`, `bandwidth_mhz`, `secondary`, `secondary_channel` (0/1/2), `data_format`
//!   (the 0.11 vocabulary), `mcs`, `stbc`, `sgi`, `antenna`, `not_sounding`, `aggregation`,
//!   `n_rx`, `n_ss`.
//! - **Stimulus:** `stimulus`, `setup_id`, `instance_id`, `dialog_token`.
//! - **Header digest:** `frame_control`, `addr1`, `addr3`, `seq_ctrl`, `retry`.
//! - **Payload:** `payload` (`esp_raw` / `grouped` / `variation`), `layout`,
//!   `first_word_invalid`, `csi_data_len`, `csi_data` (`List<Int8>`, interleaved imag/real),
//!   `subcarrier_index` (`List<Int16>`) and `subcarrier_freq_hz` (`List<Int32>`, offset from
//!   the centre) where the layout is known, `variation`, and the `grouped_*` report fields.
//! - **Vendor:** the classic-MAC (`rate`, `sig_mode`, `smoothing`, `fec_coding`, `ampdu_cnt`)
//!   and HE-MAC (`cur_bb_format`, `rx_channel_estimate_*`, `dump_len`, `is_group`,
//!   `rxend_state`, `rxmatch0..3`, `he_siga1/2`, `sigb_len`, `cur_single_mpdu`) fields.
//!
//! Schema 1 (csi-webclient ≤ 0.3) had the `dt_*`, `bandwidth` and `second` columns; they are gone
//! (`date_time` was never set, `bandwidth_mhz` and `secondary` replace the others).
//!
//! ## Durability
//! Parquet is only readable once its footer is written by [`ParquetSink::finish`]
//! (called on drop). A clean stop closes the file. An abrupt crash leaves the
//! in-progress file without a footer (and any unflushed rows lost) — that file
//! will not open.

use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use arrow::array::{
    ArrayRef, BinaryArray, BooleanArray, Int8Builder, Int16Array, Int16Builder, Int32Array,
    Int32Builder, ListBuilder, StringArray, TimestampMicrosecondArray, UInt8Array, UInt16Array,
    UInt32Array, UInt64Array,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::metadata::KeyValue;
use parquet::file::properties::WriterProperties;

use crate::export::csi::{self, CsiRecord, StreamFormat};
use crate::profile::ClientProfile;
use crate::wire::{self, Stimulus, VendorRx};

/// Version of the column set written by this sink. Bump it on any column change.
pub const SCHEMA_VERSION: u32 = 2;

/// Number of buffered rows that triggers a row-group flush.
const ROW_GROUP_SIZE: usize = 256;

/// A buffered row: the host receive time (UTC microseconds) and the record.
struct Row {
    host_rx_micros: i64,
    rec: CsiRecord,
}

/// Writes decoded CSI records to a Parquet file for one recording session.
///
/// The file footer is written by [`ParquetSink::finish`] or automatically on
/// drop, so a sink dropped at session end / disconnect / shutdown still yields a
/// readable file. Only a hard crash/panic skips finalization.
pub struct ParquetSink {
    /// `None` once finalized; `Some` while open.
    writer: Option<ArrowWriter<File>>,
    schema: Arc<Schema>,
    chip: String,
    path: String,
    buffer: Vec<Row>,
    /// Labels the `data_format` column from the numeric `cur_bb_format`.
    profile: Arc<dyn ClientProfile>,
}

impl ParquetSink {
    /// Open a new Parquet file at `path` for a session on the given `chip`.
    pub fn open(
        path: &str,
        chip: &str,
        profile: Arc<dyn ClientProfile>,
    ) -> Result<Self, ParquetSinkError> {
        let schema = build_schema();
        let file = File::create(path)?;
        let kv = metadata()
            .into_iter()
            .map(|(k, v)| KeyValue::new(k, v))
            .collect();
        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .set_key_value_metadata(Some(kv))
            .build();
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;
        Ok(Self {
            writer: Some(writer),
            schema,
            chip: chip.to_string(),
            path: path.to_string(),
            buffer: Vec::with_capacity(ROW_GROUP_SIZE),
            profile,
        })
    }

    /// The output file path this sink writes to.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Append one decoded record stamped with the host receive time
    /// (UTC microseconds). Flushes a row group once the buffer is full.
    pub fn push(&mut self, rec: CsiRecord, host_rx_micros: i64) -> Result<(), ParquetSinkError> {
        self.buffer.push(Row { host_rx_micros, rec });
        if self.buffer.len() >= ROW_GROUP_SIZE {
            self.flush()?;
        }
        Ok(())
    }

    /// Write any buffered rows as a row group.
    pub fn flush(&mut self) -> Result<(), ParquetSinkError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let columns = columns(&self.buffer, &self.chip, self.profile.as_ref())
            .into_iter()
            .map(|(_, a)| a)
            .collect();
        let batch = RecordBatch::try_new(self.schema.clone(), columns)?;
        if let Some(writer) = self.writer.as_mut() {
            writer.write(&batch)?;
        }
        self.buffer.clear();
        Ok(())
    }

    /// Flush remaining rows and write the Parquet footer. Idempotent.
    ///
    /// Called automatically on drop; the file is unreadable until this runs.
    pub fn finish(&mut self) -> Result<(), ParquetSinkError> {
        if self.writer.is_none() {
            return Ok(());
        }
        self.flush()?;
        if let Some(writer) = self.writer.take() {
            writer.close()?;
        }
        Ok(())
    }
}

impl Drop for ParquetSink {
    fn drop(&mut self) {
        // Finalize on drop so a sink abandoned via stop / disconnect / shutdown
        // still yields a readable file. Errors can only be logged here.
        if self.writer.is_some() {
            if let Err(e) = self.finish() {
                eprintln!("Failed to finalize Parquet file {}: {e}", self.path);
            }
        }
    }
}

/// Format a 6-byte MAC as `aa:bb:cc:dd:ee:ff`.
fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

/// The file-level key-value metadata.
fn metadata() -> Vec<(String, String)> {
    vec![
        ("schema_version".into(), SCHEMA_VERSION.to_string()),
        ("wire_version".into(), wire::WIRE_VERSION.to_string()),
        (
            "producer".into(),
            concat!("csi-webclient ", env!("CARGO_PKG_VERSION")).into(),
        ),
    ]
}

/// The Arrow schema: the fields [`columns`] produces, plus the file metadata.
fn build_schema() -> Arc<Schema> {
    let fields: Vec<Field> = columns(&[], "", &crate::profile::StandardClientProfile)
        .into_iter()
        .map(|(f, _)| f)
        .collect();
    let meta: HashMap<String, String> = metadata().into_iter().collect();
    Arc::new(Schema::new_with_metadata(fields, meta))
}

/// Accumulates `(field, array)` pairs so each column's name, type and values are declared once.
struct Cols<'a> {
    rows: &'a [Row],
    out: Vec<(Field, ArrayRef)>,
}

macro_rules! prim {
    ($req:ident, $opt:ident, $ty:ty, $arr:ty, $dt:expr) => {
        fn $req(&mut self, name: &str, f: impl Fn(&Row) -> $ty) {
            let a = <$arr>::from_iter_values(self.rows.iter().map(f));
            self.out.push((Field::new(name, $dt, false), Arc::new(a)));
        }
        fn $opt(&mut self, name: &str, f: impl Fn(&Row) -> Option<$ty>) {
            let a = self.rows.iter().map(f).collect::<$arr>();
            self.out.push((Field::new(name, $dt, true), Arc::new(a)));
        }
    };
}

// Not every type needs both the required and the nullable form.
#[allow(dead_code)]
impl Cols<'_> {
    prim!(u8_req, u8_opt, u8, UInt8Array, DataType::UInt8);
    prim!(u16_req, u16_opt, u16, UInt16Array, DataType::UInt16);
    prim!(u32_req, u32_opt, u32, UInt32Array, DataType::UInt32);
    prim!(u64_req, u64_opt, u64, UInt64Array, DataType::UInt64);
    prim!(i16_req, i16_opt, i16, Int16Array, DataType::Int16);
    prim!(i32_req, i32_opt, i32, Int32Array, DataType::Int32);

    fn str_req(&mut self, name: &str, f: impl Fn(&Row) -> String) {
        let a = self.rows.iter().map(|r| Some(f(r))).collect::<StringArray>();
        self.out.push((Field::new(name, DataType::Utf8, false), Arc::new(a)));
    }
    fn str_opt(&mut self, name: &str, f: impl Fn(&Row) -> Option<String>) {
        let a = self.rows.iter().map(f).collect::<StringArray>();
        self.out.push((Field::new(name, DataType::Utf8, true), Arc::new(a)));
    }
    fn bool_opt(&mut self, name: &str, f: impl Fn(&Row) -> Option<bool>) {
        let a = self.rows.iter().map(f).collect::<BooleanArray>();
        self.out.push((Field::new(name, DataType::Boolean, true), Arc::new(a)));
    }
    fn ts(&mut self, name: &str, nullable: bool, f: impl Fn(&Row) -> Option<i64>) {
        let a = self.rows.iter().map(f).collect::<TimestampMicrosecondArray>().with_timezone("UTC");
        let dt = DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into()));
        self.out.push((Field::new(name, dt, nullable), Arc::new(a)));
    }
    fn binary_opt(&mut self, name: &str, f: impl Fn(&Row) -> Option<Vec<u8>>) {
        let a = self.rows.iter().map(f).collect::<BinaryArray>();
        self.out.push((Field::new(name, DataType::Binary, true), Arc::new(a)));
    }
}

fn list_field(name: &str, item: DataType, nullable: bool) -> Field {
    Field::new(name, DataType::List(Arc::new(Field::new_list_field(item, true))), nullable)
}

/// The `EspClassic` vendor fields as `(rate, sig_mode, smoothing, fec_ldpc, ampdu_cnt)`.
fn classic(r: &Row) -> Option<(u8, u8, bool, bool, u8)> {
    match r.rec.frame.meta.vendor {
        VendorRx::EspClassic { rate, sig_mode, smoothing, fec_ldpc, ampdu_cnt } => {
            Some((rate, sig_mode, smoothing, fec_ldpc, ampdu_cnt))
        }
        _ => None,
    }
}

/// The `EspHe` vendor fields, by name.
struct He {
    rate: u8,
    cur_bb_format: u8,
    estimate_valid: bool,
    estimate_len: u16,
    dump_len: u16,
    is_group: bool,
    rxend_state: u8,
    rxmatch: u8,
    he_siga1: u32,
    he_siga2: u16,
    sigb_len: u8,
    single_mpdu: bool,
}

fn he(r: &Row) -> Option<He> {
    match r.rec.frame.meta.vendor {
        VendorRx::EspHe {
            rate,
            cur_bb_format,
            estimate_valid,
            estimate_len,
            dump_len,
            is_group,
            rxend_state,
            rxmatch,
            he_siga1,
            he_siga2,
            sigb_len,
            single_mpdu,
        } => Some(He {
            rate,
            cur_bb_format,
            estimate_valid,
            estimate_len,
            dump_len,
            is_group,
            rxend_state,
            rxmatch,
            he_siga1,
            he_siga2,
            sigb_len,
            single_mpdu,
        }),
        _ => None,
    }
}

/// Every column, in file order, projected from `rows`. The single source of the schema: called with
/// no rows it yields the field list.
fn columns(rows: &[Row], session_chip: &str, profile: &dyn ClientProfile) -> Vec<(Field, ArrayRef)> {
    let mut c = Cols { rows, out: Vec::with_capacity(80) };
    let b = |v: bool| u32::from(v);

    // ── Provenance ──────────────────────────────────────────────────────
    c.ts("host_rx_time", false, |r| Some(r.host_rx_micros));
    c.str_req("chip", |r| {
        r.rec.chip.map_or_else(|| session_chip.to_string(), |ch| csi::chip_name(ch).to_string())
    });
    c.str_req("stream_format", |r| {
        match r.rec.format {
            StreamFormat::Wire => "wire",
            StreamFormat::Legacy => "legacy",
        }
        .to_string()
    });
    c.u8_opt("wire_version", |r| r.rec.envelope.map(|e| e.version));
    c.str_opt("node_id", |r| r.rec.envelope.map(|e| format_mac(&e.node_id)));
    c.u32_opt("session_id", |r| r.rec.envelope.map(|e| e.session_id));
    c.u32_opt("stream_seq", |r| r.rec.envelope.map(|e| e.stream_seq));
    c.str_opt("source", |r| r.rec.envelope.map(|e| csi::source_name(e.source).to_string()));

    // ── Time ────────────────────────────────────────────────────────────
    c.u64_req("timestamp_us", |r| r.rec.frame.meta.timestamp_us);
    c.u32_req("timestamp", |r| r.rec.frame.meta.timestamp_us as u32);
    c.ts("device_time", true, |r| r.rec.device_time_unix_us);

    // ── Receive metadata ────────────────────────────────────────────────
    c.str_opt("mac", |r| r.rec.frame.transmitter().map(|m| format_mac(&m)));
    c.i32_req("rssi", |r| i32::from(r.rec.frame.meta.rssi));
    c.i32_req("noise_floor", |r| i32::from(r.rec.frame.meta.noise_floor));
    c.u32_req("channel", |r| u32::from(r.rec.frame.meta.channel));
    c.u32_req("sig_len", |r| u32::from(r.rec.frame.meta.sig_len));
    c.u32_req("rx_state", |r| u32::from(r.rec.frame.meta.rx_state));
    c.u16_opt("sequence_number", |r| r.rec.frame.meta.frame_seq);
    c.str_req("ppdu", |r| csi::ppdu_name(r.rec.frame.meta.ppdu).to_string());
    c.u16_opt("bandwidth_mhz", |r| r.rec.frame.meta.bandwidth.map(wire::Bandwidth::mhz));
    c.str_req("secondary", |r| csi::secondary_name(r.rec.frame.meta.secondary).to_string());
    c.u32_opt("secondary_channel", |r| Some(csi::secondary_code(r.rec.frame.meta.secondary)));
    // Prefer a profile-supplied label for the numeric `cur_bb_format`.
    c.str_req("data_format", |r| {
        r.rec
            .cur_bb_format()
            .and_then(|f| profile.label_format(u32::from(f)))
            .unwrap_or_else(|| r.rec.data_format().as_str())
            .to_string()
    });
    c.u32_opt("mcs", |r| r.rec.frame.meta.mcs.map(u32::from));
    c.u32_opt("stbc", |r| r.rec.frame.meta.stbc.map(b));
    c.u32_opt("sgi", |r| r.rec.frame.meta.sgi.map(b));
    c.u32_opt("antenna", |r| r.rec.frame.meta.antenna.map(u32::from));
    c.u32_opt("not_sounding", |r| r.rec.frame.meta.not_sounding.map(b));
    c.u32_opt("aggregation", |r| r.rec.frame.meta.aggregation.map(b));
    c.u8_req("n_rx", |r| r.rec.frame.meta.n_rx);
    c.u8_opt("n_ss", |r| r.rec.frame.meta.n_ss);

    // ── Stimulus ────────────────────────────────────────────────────────
    c.str_req("stimulus", |r| csi::stimulus_name(&r.rec.frame.stimulus).to_string());
    c.u8_opt("setup_id", |r| match r.rec.frame.stimulus {
        Stimulus::Controlled { setup_id, .. } => Some(setup_id),
        _ => None,
    });
    c.u16_opt("instance_id", |r| match r.rec.frame.stimulus {
        Stimulus::Controlled { instance_id, .. } => Some(instance_id),
        _ => None,
    });
    c.u8_opt("dialog_token", |r| match r.rec.frame.stimulus {
        Stimulus::Observed { dialog_token, .. } => Some(dialog_token),
        _ => None,
    });

    // ── Header digest ───────────────────────────────────────────────────
    c.u16_opt("frame_control", |r| r.rec.frame.header.map(|h| h.frame_control));
    c.str_opt("addr1", |r| r.rec.frame.header.map(|h| format_mac(&h.addr1)));
    c.str_opt("addr3", |r| r.rec.frame.header.map(|h| format_mac(&h.addr3)));
    c.u16_opt("seq_ctrl", |r| r.rec.frame.header.map(|h| h.seq_ctrl));
    c.bool_opt("retry", |r| csi::header_retry(&r.rec.frame.header));

    // ── Payload ─────────────────────────────────────────────────────────
    c.str_req("payload", |r| csi::payload_name(&r.rec.frame.payload).to_string());
    c.str_opt("layout", |r| r.rec.layout().map(|l| format!("{l:?}")));
    c.bool_opt("first_word_invalid", |r| r.rec.first_word_invalid());
    c.u16_req("csi_data_len", |r| r.rec.csi_data().len() as u16);

    let mut csi_data = ListBuilder::new(Int8Builder::new());
    let mut sc_index = ListBuilder::new(Int16Builder::new());
    let mut sc_freq = ListBuilder::new(Int32Builder::new());
    for r in rows {
        csi_data.values().append_slice(r.rec.csi_data());
        csi_data.append(true);
        match r.rec.subcarrier_indices() {
            Some(v) => {
                sc_index.values().append_slice(&v);
                sc_index.append(true);
            }
            None => sc_index.append(false),
        }
        match r.rec.subcarrier_freqs_hz() {
            Some(v) => {
                sc_freq.values().append_slice(&v);
                sc_freq.append(true);
            }
            None => sc_freq.append(false),
        }
    }
    c.out.push((list_field("csi_data", DataType::Int8, false), Arc::new(csi_data.finish())));
    c.out.push((list_field("subcarrier_index", DataType::Int16, true), Arc::new(sc_index.finish())));
    c.out.push((list_field("subcarrier_freq_hz", DataType::Int32, true), Arc::new(sc_freq.finish())));

    c.u16_opt("variation", |r| r.rec.variation());
    c.u8_opt("grouped_ng", |r| r.rec.grouped().map(|g| g.ng));
    c.u8_opt("grouped_nb", |r| r.rec.grouped().map(|g| g.nb));
    c.i16_opt("grouped_sc_start", |r| r.rec.grouped().map(|g| g.sc_start));
    c.u16_opt("grouped_n_sc", |r| r.rec.grouped().map(|g| g.n_sc));
    c.u8_opt("grouped_n_rx", |r| r.rec.grouped().map(|g| g.n_rx));
    c.u8_opt("grouped_n_tx", |r| r.rec.grouped().map(|g| g.n_tx));
    c.binary_opt("grouped_data", |r| r.rec.grouped().map(|g| g.data));

    // ── Vendor: classic MAC (rate is shared with the HE MAC) ────────────
    c.u32_opt("rate", |r| {
        classic(r).map(|(rate, ..)| u32::from(rate)).or_else(|| he(r).map(|h| u32::from(h.rate)))
    });
    c.u32_opt("sig_mode", |r| classic(r).map(|(_, m, ..)| u32::from(m)));
    c.u32_opt("smoothing", |r| classic(r).map(|(_, _, s, ..)| b(s)));
    c.u32_opt("fec_coding", |r| classic(r).map(|(.., f, _)| b(f)));
    c.u32_opt("ampdu_cnt", |r| classic(r).map(|(.., n)| u32::from(n)));

    // ── Vendor: HE MAC (C5 / C6) ────────────────────────────────────────
    c.u32_opt("cur_bb_format", |r| he(r).map(|h| u32::from(h.cur_bb_format)));
    c.u32_opt("rx_channel_estimate_info_vld", |r| he(r).map(|h| b(h.estimate_valid)));
    c.u32_opt("rx_channel_estimate_len", |r| he(r).map(|h| u32::from(h.estimate_len)));
    c.u32_opt("dump_len", |r| he(r).map(|h| u32::from(h.dump_len)));
    c.u32_opt("is_group", |r| he(r).map(|h| b(h.is_group)));
    c.u32_opt("rxend_state", |r| he(r).map(|h| u32::from(h.rxend_state)));
    // `rxmatch0`, `sigb_len` and `cur_single_mpdu` exist only on the C6.
    let c6 = |r: &Row| r.rec.chip == Some(wire::Chip::Esp32C6);
    for bit in 0..4u8 {
        c.u32_opt(&format!("rxmatch{bit}"), |r| {
            he(r).filter(|_| bit > 0 || c6(r)).map(|h| u32::from((h.rxmatch >> bit) & 1))
        });
    }
    // Not in the pre-0.12 packet: null for legacy frames.
    let wire_only = |r: &Row| r.rec.format == StreamFormat::Wire;
    c.u32_opt("he_siga1", |r| he(r).filter(|_| wire_only(r)).map(|h| h.he_siga1));
    c.u16_opt("he_siga2", |r| he(r).filter(|_| wire_only(r)).map(|h| h.he_siga2));
    c.u32_opt("sigb_len", |r| he(r).filter(|_| c6(r)).map(|h| u32::from(h.sigb_len)));
    c.u32_opt("cur_single_mpdu", |r| he(r).filter(|_| c6(r)).map(|h| b(h.single_mpdu)));

    c.out
}

/// Error opening, writing, or closing a Parquet session file.
#[derive(Debug)]
pub enum ParquetSinkError {
    Io(std::io::Error),
    Arrow(arrow::error::ArrowError),
    Parquet(parquet::errors::ParquetError),
}

impl std::fmt::Display for ParquetSinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "parquet sink io error: {e}"),
            Self::Arrow(e) => write!(f, "parquet sink arrow error: {e}"),
            Self::Parquet(e) => write!(f, "parquet sink error: {e}"),
        }
    }
}

impl std::error::Error for ParquetSinkError {}

impl From<std::io::Error> for ParquetSinkError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<arrow::error::ArrowError> for ParquetSinkError {
    fn from(e: arrow::error::ArrowError) -> Self {
        Self::Arrow(e)
    }
}
impl From<parquet::errors::ParquetError> for ParquetSinkError {
    fn from(e: parquet::errors::ParquetError) -> Self {
        Self::Parquet(e)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::csi::{Decoded, FrameDecoder};
    use crate::export::legacy::{DateTime, PacketA, RxCsiFmt};
    use crate::profile::StandardClientProfile;
    use crate::wire::{
        Bandwidth, Body, Chip, CsiFrame, CsiPayload, Envelope, HeaderDigest, LayoutId, PpduFormat,
        RxMeta, Secondary, SourceKind,
    };
    use arrow::array::{Array, AsArray};
    use arrow::datatypes::{Int16Type, UInt16Type, UInt64Type};
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use parquet::file::reader::{FileReader, SerializedFileReader};

    fn legacy_record() -> CsiRecord {
        let pkt = PacketA {
            mac: [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
            rssi: -50,
            timestamp: 1000,
            rate: 11,
            sgi: 1,
            secondary_channel: 0,
            channel: 6,
            bandwidth: 0,
            antenna: 0,
            sig_mode: 1,
            mcs: 7,
            smoothing: 0,
            not_sounding: 1,
            aggregation: 0,
            stbc: 0,
            fec_coding: 0,
            ampdu_cnt: 0,
            noise_floor: -95,
            rx_state: 0,
            sig_len: 100,
            date_time: Some(DateTime {
                year: 2026,
                month: 6,
                day: 22,
                hour: 1,
                minute: 2,
                second: 3,
                millisecond: 4,
            }),
            sequence_number: 1,
            data_format: RxCsiFmt::HtBw20,
            csi_data_len: 256,
            csi_data: vec![1; 256],
        };
        let mut buf = vec![0u8; 2048];
        let cobs = postcard::to_slice_cobs(&pkt, &mut buf).unwrap();
        let body = cobs.strip_suffix(&[0]).unwrap_or(cobs);
        match FrameDecoder::new(Some(Chip::Esp32)).decode(body).unwrap() {
            Decoded::Csi(r) => *r,
            Decoded::Session(..) => unreachable!(),
        }
    }

    fn wire_record() -> CsiRecord {
        let meta = RxMeta {
            timestamp_us: u64::from(u32::MAX) + 10,
            rssi: -41,
            noise_floor: -93,
            channel: 36,
            secondary: Secondary::None,
            bandwidth: Some(Bandwidth::Mhz20),
            ppdu: PpduFormat::HeSu,
            mcs: None,
            stbc: None,
            sgi: None,
            n_rx: 1,
            n_ss: None,
            antenna: None,
            sig_len: 200,
            rx_state: 0,
            not_sounding: None,
            aggregation: None,
            frame_seq: Some(12),
            vendor: VendorRx::EspHe {
                rate: 0,
                cur_bb_format: 4,
                estimate_valid: true,
                estimate_len: 490,
                dump_len: 490,
                is_group: false,
                rxend_state: 0,
                rxmatch: 0b10,
                he_siga1: 7,
                he_siga2: 8,
                sigb_len: 0,
                single_mpdu: false,
            },
        };
        let hdr = HeaderDigest {
            frame_control: 0x0888,
            addr1: [1; 6],
            addr2: [2; 6],
            addr3: [3; 6],
            seq_ctrl: 12 << 4,
        };
        let bytes = (0..490).map(|i| i as i8).collect();
        let frame = CsiFrame::new(
            meta,
            Stimulus::Controlled { setup_id: 1, instance_id: 99, ta: [2; 6] },
            Some(hdr),
            CsiPayload::EspRaw { chip: Chip::Esp32C5, layout: LayoutId::C5He20Su, first_word_invalid: true, bytes },
        );
        let env = Envelope::new([9; 6], 0xabcd, SourceKind::EspVendor, 5);
        let mut buf = vec![0u8; wire::MAX_ENCODED_LEN];
        let used = wire::encode_cobs(&env, &Body::Csi(frame), &mut buf).unwrap();
        match FrameDecoder::new(None).decode(used).unwrap() {
            Decoded::Csi(r) => *r,
            Decoded::Session(..) => unreachable!(),
        }
    }

    #[test]
    fn writes_versioned_readable_parquet() {
        let path = std::env::temp_dir().join("csi_client_sink_test_v2.parquet");
        let path_str = path.to_str().unwrap();
        {
            let mut sink =
                ParquetSink::open(path_str, "esp32c5", Arc::new(StandardClientProfile)).unwrap();
            sink.push(wire_record(), 1_700_000_000_000_000).unwrap();
            sink.push(legacy_record(), 1_700_000_000_000_001).unwrap();
        }

        // Key-value metadata carries the schema version.
        let reader = SerializedFileReader::new(File::open(path_str).unwrap()).unwrap();
        let kv = reader.metadata().file_metadata().key_value_metadata().unwrap();
        let get = |k: &str| kv.iter().find(|e| e.key == k).and_then(|e| e.value.clone());
        assert_eq!(get("schema_version"), Some(SCHEMA_VERSION.to_string()));
        assert_eq!(get("wire_version"), Some(wire::WIRE_VERSION.to_string()));

        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(path_str).unwrap()).unwrap();
        let batch = builder.build().unwrap().next().unwrap().unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), build_schema().fields().len());
        let col = |n: &str| batch.column_by_name(n).unwrap_or_else(|| panic!("no column {n}")).clone();
        let s = |n: &str, i: usize| col(n).as_string::<i32>().value(i).to_string();

        // Wire row.
        assert_eq!(s("stream_format", 0), "wire");
        assert_eq!(s("chip", 0), "esp32c5");
        assert_eq!(s("node_id", 0), "09:09:09:09:09:09");
        assert_eq!(s("mac", 0), "02:02:02:02:02:02");
        assert_eq!(s("ppdu", 0), "he_su");
        assert_eq!(s("layout", 0), "C5He20Su");
        assert_eq!(s("stimulus", 0), "controlled");
        assert_eq!(s("source", 0), "esp_vendor");
        assert_eq!(col("timestamp_us").as_primitive::<UInt64Type>().value(0), u64::from(u32::MAX) + 10);
        assert_eq!(col("instance_id").as_primitive::<UInt16Type>().value(0), 99);
        assert_eq!(col("seq_ctrl").as_primitive::<UInt16Type>().value(0), 12 << 4);
        assert!(col("first_word_invalid").as_boolean().value(0));
        let idx = col("subcarrier_index");
        let idx = idx.as_list::<i32>().value(0);
        assert_eq!(idx.len(), 245);
        assert_eq!(idx.as_primitive::<Int16Type>().value(244), -1);
        assert!(col("rxmatch0").is_null(0));
        assert!(col("sig_mode").is_null(0));

        // Legacy row.
        assert_eq!(s("stream_format", 1), "legacy");
        assert_eq!(s("data_format", 1), "HtBw20");
        assert_eq!(s("layout", 1), "ClassicNoneHt20");
        assert!(col("node_id").is_null(1));
        assert!(col("frame_control").is_null(1));
        assert!(col("cur_bb_format").is_null(1));
        assert_eq!(col("subcarrier_index").as_list::<i32>().value(1).len(), 128);
        let _ = std::fs::remove_file(path_str);
    }
}
