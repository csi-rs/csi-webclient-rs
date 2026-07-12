//! Local CSI stream export.
//!
//! Decodes raw WebSocket CSI frames (COBS-framed postcard records, identical to
//! what `csi-webserver-rs` consumes) and writes them to a Parquet file on the
//! client host. The output schema matches the server's `csi_dump_*.parquet`, so
//! a locally-recorded file and a server-side dump are interchangeable.

pub mod csi;
pub mod parquet_sink;

use std::sync::Arc;

use csi::ChipVariant;
use parquet_sink::{ParquetSink, ParquetSinkError};

use crate::profile::ClientProfile;

/// An active recording of one device's CSI stream to a Parquet file.
///
/// Decodes each incoming frame against the device's chip layout and appends it
/// to the sink. Lives outside [`crate::state::AppState`] (which is `Clone`)
/// because the underlying file writer is not cloneable.
pub struct Recorder {
    sink: ParquetSink,
    chip: ChipVariant,
    /// Frames successfully decoded and written.
    pub frames_written: u64,
    /// Frames that failed to decode (wire drift, truncation, wrong chip).
    pub decode_errors: u64,
}

impl Recorder {
    /// Open a Parquet file at `path` for a stream from the given `chip` string.
    ///
    /// `profile` labels the `data_format` column from the numeric
    /// `cur_bb_format` where it can (see [`ClientProfile::label_format`]).
    ///
    /// Returns `Err` if the chip is unrecognized (no known wire layout) or the
    /// file cannot be created.
    pub fn start(
        path: &str,
        chip: &str,
        profile: Arc<dyn ClientProfile>,
    ) -> Result<Self, String> {
        let variant = ChipVariant::from_chip_str(chip)
            .ok_or_else(|| format!("unknown chip '{chip}'; cannot decode CSI frames"))?;
        let sink = ParquetSink::open(path, chip, profile).map_err(|e| e.to_string())?;
        Ok(Self {
            sink,
            chip: variant,
            frames_written: 0,
            decode_errors: 0,
        })
    }

    /// The output file path being written.
    pub fn path(&self) -> &str {
        self.sink.path()
    }

    /// Decode one raw WebSocket frame and append it, stamped with `host_rx_micros`
    /// (UTC microseconds). Decode failures are counted, not propagated, so a
    /// single malformed frame never aborts a recording.
    pub fn record_frame(&mut self, bytes: &[u8], host_rx_micros: i64) -> Result<(), ParquetSinkError> {
        match csi::decode(bytes, self.chip) {
            Ok(decoded) => {
                self.sink.push(decoded, host_rx_micros)?;
                self.frames_written += 1;
            }
            Err(_) => {
                self.decode_errors += 1;
            }
        }
        Ok(())
    }

    /// Flush remaining rows and finalize the Parquet footer.
    pub fn finish(mut self) -> Result<(), ParquetSinkError> {
        self.sink.finish()
    }
}
