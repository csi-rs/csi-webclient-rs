//! Host-side decoder for the firmware's `serialized` CSI output.
//!
//! WebSocket frames from `csi-webserver-rs` are the device's serialized records, one COBS frame each
//! (the server strips the trailing `\0`). esp-csi-rs 0.12 encodes them in the versioned
//! [`crate::wire`] format: `postcard(Envelope) ++ postcard(Body)`. Firmware from 0.8 to 0.11 sent a
//! chip-specific `CSIDataPacket` instead, which [`super::legacy`] still decodes when the chip is
//! known.
//!
//! [`FrameDecoder`] tries the wire format first and falls back to the legacy layout, then sticks to
//! whichever format the stream turned out to use. The two cannot be confused on a real stream: a
//! wire frame starts with [`wire::WIRE_VERSION`] (`0x01`), while a legacy frame starts with the
//! transmitter MAC, whose first octet never has the group bit (`0x01`) set.

use std::collections::HashMap;

use super::legacy::{self, RxCsiFmt};
use crate::wire::{
    self, Body, Chip, CsiFrame, CsiPayload, Envelope, HeaderDigest, LayoutId, PpduFormat, RxMeta,
    SessionInfo, SourceKind, Stimulus, VendorRx,
};

/// Which serialized format a stream uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFormat {
    /// esp-csi-rs 0.12+: the versioned [`crate::wire`] format.
    Wire,
    /// esp-csi-rs 0.8 – 0.11: the chip-specific `CSIDataPacket`.
    Legacy,
}

/// A grouped, quantised channel report ([`CsiPayload::Grouped`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupedReport {
    /// Subcarrier grouping.
    pub ng: u8,
    /// Bits per quantised value.
    pub nb: u8,
    /// Index of the first reported subcarrier.
    pub sc_start: i16,
    /// Number of reported subcarriers.
    pub n_sc: u16,
    /// Receive chains.
    pub n_rx: u8,
    /// Transmit chains.
    pub n_tx: u8,
    /// Packed report.
    pub data: Vec<u8>,
}

/// One decoded CSI measurement, whatever format it arrived in. This is what the Parquet sink
/// writes.
#[derive(Debug, Clone)]
pub struct CsiRecord {
    /// The format the frame arrived in.
    pub format: StreamFormat,
    /// The frame's envelope. `None` for a legacy frame.
    pub envelope: Option<Envelope>,
    /// The measurement, in wire terms. A legacy frame is mapped onto these types.
    pub frame: CsiFrame,
    /// The chip that produced the measurement: from an `EspRaw` payload, else from the session
    /// announcement.
    pub chip: Option<Chip>,
    /// Wall-clock receive time (UNIX epoch, microseconds), when the session announcement anchored
    /// the node's clock.
    pub device_time_unix_us: Option<i64>,
    /// A legacy packet's own `data_format`.
    pub legacy_format: Option<RxCsiFmt>,
}

impl CsiRecord {
    /// The raw CSI buffer: `EspRaw` bytes, empty for other payloads.
    pub fn csi_data(&self) -> &[i8] {
        match &self.frame.payload {
            CsiPayload::EspRaw { bytes, .. } => bytes,
            _ => &[],
        }
    }

    /// The `EspRaw` buffer's layout.
    pub fn layout(&self) -> Option<LayoutId> {
        match &self.frame.payload {
            CsiPayload::EspRaw { layout, .. } => Some(*layout),
            _ => None,
        }
    }

    /// The `EspRaw` buffer's first-word-invalid flag.
    pub fn first_word_invalid(&self) -> Option<bool> {
        match &self.frame.payload {
            CsiPayload::EspRaw { first_word_invalid, .. } => Some(*first_word_invalid),
            _ => None,
        }
    }

    /// The `Variation` payload's value.
    pub fn variation(&self) -> Option<u16> {
        match &self.frame.payload {
            CsiPayload::Variation { value } => Some(*value),
            _ => None,
        }
    }

    /// The `Grouped` payload.
    pub fn grouped(&self) -> Option<GroupedReport> {
        match &self.frame.payload {
            CsiPayload::Grouped { ng, nb, sc_start, n_sc, n_rx, n_tx, data } => Some(GroupedReport {
                ng: *ng,
                nb: *nb,
                sc_start: *sc_start,
                n_sc: *n_sc,
                n_rx: *n_rx,
                n_tx: *n_tx,
                data: data.to_vec(),
            }),
            _ => None,
        }
    }

    /// `cur_bb_format` from [`VendorRx::EspHe`].
    pub fn cur_bb_format(&self) -> Option<u8> {
        match self.frame.meta.vendor {
            VendorRx::EspHe { cur_bb_format, .. } => Some(cur_bb_format),
            _ => None,
        }
    }

    /// The 0.11-vocabulary `data_format` name: the legacy packet's own, else derived from the
    /// metadata.
    pub fn data_format(&self) -> RxCsiFmt {
        self.legacy_format
            .unwrap_or_else(|| legacy::data_format_of(&self.frame.meta))
    }

    /// Subcarrier indices, one per reported value, in buffer order. Derived from the `EspRaw`
    /// [`LayoutId`] (`None` when it is `Unknown` or disagrees with the buffer length), or from a
    /// `Grouped` report's start and grouping.
    pub fn subcarrier_indices(&self) -> Option<Vec<i16>> {
        match &self.frame.payload {
            CsiPayload::EspRaw { layout, bytes, .. } => {
                (layout.byte_len() == Some(bytes.len()))
                    .then(|| layout.indices().map(|(_, i)| i).collect())
            }
            CsiPayload::Grouped { ng, sc_start, n_sc, .. } => Some(
                (0..*n_sc)
                    .map(|k| sc_start.saturating_add((k as i16).saturating_mul(*ng as i16)))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Subcarrier frequency offsets from the centre the indices refer to, Hz: index × the training
    /// field's spacing. Same length and order as [`Self::subcarrier_indices`]. A `Grouped` report
    /// uses the HE spacing for an HE PPDU and the legacy spacing otherwise.
    pub fn subcarrier_freqs_hz(&self) -> Option<Vec<i32>> {
        match &self.frame.payload {
            CsiPayload::EspRaw { layout, bytes, .. } => (layout.byte_len() == Some(bytes.len()))
                .then(|| {
                    layout
                        .indices()
                        .map(|(ltf, i)| i32::from(i) * ltf.spacing_hz() as i32)
                        .collect()
                }),
            CsiPayload::Grouped { .. } => {
                let spacing = if is_he(self.frame.meta.ppdu) {
                    wire::layout::SPACING_HZ_HE
                } else {
                    wire::layout::SPACING_HZ_LEGACY
                } as i32;
                self.subcarrier_indices()
                    .map(|v| v.into_iter().map(|i| i32::from(i) * spacing).collect())
            }
            _ => None,
        }
    }
}

fn is_he(p: PpduFormat) -> bool {
    matches!(p, PpduFormat::HeSu | PpduFormat::HeMu | PpduFormat::HeErSu | PpduFormat::HeTb)
}

/// What one frame decoded to.
#[derive(Debug, Clone)]
pub enum Decoded {
    /// A measurement.
    Csi(Box<CsiRecord>),
    /// A session announcement (wire format only).
    Session(Envelope, SessionInfo),
}

/// Failure decoding a serialized frame.
#[derive(Debug)]
pub enum DecodeError {
    /// The frame is not a valid wire frame (and no legacy fallback applied).
    Wire(wire::DecodeError),
    /// The stream is in the legacy format and this frame did not decode.
    Legacy(postcard::Error),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wire(wire::DecodeError::UnsupportedVersion(v)) => write!(
                f,
                "unsupported wire version {v} (this client decodes version {})",
                wire::WIRE_VERSION
            ),
            Self::Wire(wire::DecodeError::Malformed(e)) => {
                write!(f, "failed to decode CSI frame: {e}")
            }
            Self::Legacy(e) => write!(f, "failed to decode legacy CSI frame: {e}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decodes one device's frames, remembering the stream's format and its session announcements.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    legacy_chip: Option<Chip>,
    format: Option<StreamFormat>,
    /// Session announcements by `(node_id, session_id)`.
    sessions: HashMap<([u8; 6], u32), SessionInfo>,
}

impl FrameDecoder {
    /// A decoder for one device's stream. `legacy_chip` enables the pre-0.12 fallback, whose layout
    /// depends on the chip; with `None` only wire frames decode.
    pub fn new(legacy_chip: Option<Chip>) -> Self {
        Self {
            legacy_chip,
            ..Self::default()
        }
    }

    /// The format the stream turned out to use, once a frame has decoded.
    pub fn format(&self) -> Option<StreamFormat> {
        self.format
    }

    /// Decode one WebSocket frame (a COBS frame, with or without its trailing `\0`).
    pub fn decode(&mut self, frame: &[u8]) -> Result<Decoded, DecodeError> {
        match self.format {
            Some(StreamFormat::Wire) => self.decode_wire(frame).map_err(DecodeError::Wire),
            Some(StreamFormat::Legacy) => self.decode_legacy(frame),
            None => match self.decode_wire(frame) {
                Ok(d) => {
                    self.format = Some(StreamFormat::Wire);
                    Ok(d)
                }
                Err(wire_err) => {
                    if self.legacy_chip.is_none() {
                        return Err(DecodeError::Wire(wire_err));
                    }
                    let d = self.decode_legacy(frame).map_err(|_| DecodeError::Wire(wire_err))?;
                    self.format = Some(StreamFormat::Legacy);
                    Ok(d)
                }
            },
        }
    }

    fn decode_wire(&mut self, frame: &[u8]) -> Result<Decoded, wire::DecodeError> {
        let mut owned = frame.to_vec();
        let (envelope, body) = wire::decode_cobs(&mut owned)?;
        match body {
            Body::Session(info) => {
                self.sessions.insert((envelope.node_id, envelope.session_id), info);
                Ok(Decoded::Session(envelope, info))
            }
            Body::Csi(frame) => {
                let session = self.sessions.get(&(envelope.node_id, envelope.session_id));
                let chip = match &frame.payload {
                    CsiPayload::EspRaw { chip, .. } => Some(*chip),
                    _ => session.map(|s| s.chip),
                };
                let device_time_unix_us = session.and_then(|s| wall_time(s, &frame.meta));
                Ok(Decoded::Csi(Box::new(CsiRecord {
                    format: StreamFormat::Wire,
                    envelope: Some(envelope),
                    frame,
                    chip,
                    device_time_unix_us,
                    legacy_format: None,
                })))
            }
        }
    }

    fn decode_legacy(&self, frame: &[u8]) -> Result<Decoded, DecodeError> {
        let Some(chip) = self.legacy_chip else {
            return Err(DecodeError::Legacy(postcard::Error::DeserializeBadEncoding));
        };
        let decoded = legacy::decode(frame, chip).map_err(DecodeError::Legacy)?;
        Ok(Decoded::Csi(Box::new(CsiRecord {
            format: StreamFormat::Legacy,
            envelope: None,
            frame: decoded.frame,
            chip: Some(chip),
            device_time_unix_us: None,
            legacy_format: Some(decoded.data_format),
        })))
    }
}

/// `meta`'s receive time on the wall clock, from the session's anchor.
fn wall_time(session: &SessionInfo, meta: &RxMeta) -> Option<i64> {
    let epoch = i128::from(session.epoch_unix_us?);
    let t = epoch + i128::from(meta.timestamp_us) - i128::from(session.timestamp_us);
    i64::try_from(t).ok()
}

/// Stable name of a [`SourceKind`] for the export.
pub fn source_name(s: SourceKind) -> &'static str {
    match s {
        SourceKind::EspVendor => "esp_vendor",
        SourceKind::Ieee80211bf => "ieee80211bf",
        SourceKind::Synthetic => "synthetic",
    }
}

/// Stable name of a [`PpduFormat`] for the export.
pub fn ppdu_name(p: PpduFormat) -> &'static str {
    match p {
        PpduFormat::Unknown => "unknown",
        PpduFormat::Dsss => "dsss",
        PpduFormat::NonHt => "non_ht",
        PpduFormat::Ht => "ht",
        PpduFormat::Vht => "vht",
        PpduFormat::VhtMu => "vht_mu",
        PpduFormat::HeSu => "he_su",
        PpduFormat::HeMu => "he_mu",
        PpduFormat::HeErSu => "he_er_su",
        PpduFormat::HeTb => "he_tb",
        PpduFormat::Eht => "eht",
    }
}

/// Stable name of a [`wire::Secondary`] for the export.
pub fn secondary_name(s: wire::Secondary) -> &'static str {
    match s {
        wire::Secondary::None => "none",
        wire::Secondary::Above => "above",
        wire::Secondary::Below => "below",
    }
}

/// The 0.11 numeric secondary-channel code (0 none, 1 above, 2 below).
pub fn secondary_code(s: wire::Secondary) -> u32 {
    match s {
        wire::Secondary::None => 0,
        wire::Secondary::Above => 1,
        wire::Secondary::Below => 2,
    }
}

/// Stable name of a [`Stimulus`] kind for the export.
pub fn stimulus_name(s: &Stimulus) -> &'static str {
    match s {
        Stimulus::Controlled { .. } => "controlled",
        Stimulus::Ambient { .. } => "ambient",
        Stimulus::Observed { .. } => "observed",
    }
}

/// Stable name of a [`CsiPayload`] kind for the export.
pub fn payload_name(p: &CsiPayload) -> &'static str {
    match p {
        CsiPayload::EspRaw { .. } => "esp_raw",
        CsiPayload::Grouped { .. } => "grouped",
        CsiPayload::Variation { .. } => "variation",
    }
}

/// Firmware `chip=` string of a [`Chip`].
pub fn chip_name(c: Chip) -> &'static str {
    match c {
        Chip::Esp32 => "esp32",
        Chip::Esp32S2 => "esp32s2",
        Chip::Esp32S3 => "esp32s3",
        Chip::Esp32C3 => "esp32c3",
        Chip::Esp32C5 => "esp32c5",
        Chip::Esp32C6 => "esp32c6",
    }
}

/// The header digest's Retry bit, when a digest is present.
pub fn header_retry(h: &Option<HeaderDigest>) -> Option<bool> {
    h.as_ref().map(HeaderDigest::is_retry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Bandwidth, Secondary};
    use heapless::Vec as HVec;

    fn meta() -> RxMeta {
        RxMeta {
            timestamp_us: 5_000_000_123,
            rssi: -40,
            noise_floor: -92,
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
            sig_len: 120,
            rx_state: 0,
            not_sounding: None,
            aggregation: None,
            frame_seq: Some(77),
            vendor: VendorRx::EspHe {
                rate: 0,
                cur_bb_format: 4,
                estimate_valid: true,
                estimate_len: 490,
                dump_len: 490,
                is_group: false,
                rxend_state: 0,
                rxmatch: 0b10,
                he_siga1: 0x1234,
                he_siga2: 0x56,
                sigb_len: 0,
                single_mpdu: false,
            },
        }
    }

    fn encode(env: &Envelope, body: &Body) -> Vec<u8> {
        let mut buf = vec![0u8; wire::MAX_ENCODED_LEN];
        let used = wire::encode_cobs(env, body, &mut buf).unwrap();
        // The server strips the delimiter.
        used.strip_suffix(&[0]).unwrap().to_vec()
    }

    #[test]
    fn decodes_wire_he20_frame_with_session_anchor() {
        let node = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        let mut dec = FrameDecoder::new(Some(Chip::Esp32C5));

        let info = SessionInfo::new(Chip::Esp32C5, [0, 12, 0], 5_000_000_000, Some(1_700_000_000_000_000));
        let s = encode(&Envelope::new(node, 9, SourceKind::EspVendor, 0), &Body::Session(info));
        assert!(matches!(dec.decode(&s).unwrap(), Decoded::Session(_, _)));
        assert_eq!(dec.format(), Some(StreamFormat::Wire));

        let bytes: HVec<i8, { wire::MAX_CSI_BYTES }> = (0..490).map(|i| (i % 100) as i8).collect();
        let frame = CsiFrame::new(
            meta(),
            Stimulus::Controlled { setup_id: 3, instance_id: 41, ta: [2, 0, 0, 0, 0, 1] },
            None,
            CsiPayload::EspRaw { chip: Chip::Esp32C5, layout: LayoutId::C5He20Su, first_word_invalid: false, bytes },
        );
        let f = encode(&Envelope::new(node, 9, SourceKind::EspVendor, 1), &Body::Csi(frame));
        let Decoded::Csi(rec) = dec.decode(&f).unwrap() else { panic!("expected csi") };
        assert_eq!(rec.cur_bb_format(), Some(4));
        assert_eq!(rec.csi_data().len(), 490);
        assert_eq!(rec.device_time_unix_us, Some(1_700_000_000_000_123));
        let idx = rec.subcarrier_indices().unwrap();
        assert_eq!(idx.len(), 245);
        assert_eq!((idx[0], idx[122], idx[123], idx[244]), (0, 122, -122, -1));
        let hz = rec.subcarrier_freqs_hz().unwrap();
        assert_eq!(hz[1], 78_125);
        assert_eq!(rec.frame.transmitter(), Some([2, 0, 0, 0, 0, 1]));
    }

    #[test]
    fn grouped_indices_follow_start_and_grouping() {
        let frame = CsiFrame::new(
            meta(),
            Stimulus::Ambient { ta: [0; 6] },
            None,
            CsiPayload::Grouped { ng: 4, nb: 8, sc_start: -122, n_sc: 3, n_rx: 1, n_tx: 1, data: HVec::from_slice(&[1, 2, 3]).unwrap() },
        );
        let f = encode(&Envelope::new([0; 6], 1, SourceKind::Synthetic, 0), &Body::Csi(frame));
        let Decoded::Csi(rec) = FrameDecoder::new(None).decode(&f).unwrap() else { panic!() };
        assert_eq!(rec.subcarrier_indices().unwrap(), vec![-122, -118, -114]);
        assert_eq!(rec.subcarrier_freqs_hz().unwrap()[1], -118 * 78_125);
        assert_eq!(rec.grouped().unwrap().data, vec![1, 2, 3]);
        assert!(rec.csi_data().is_empty());
    }

    #[test]
    fn rejects_other_wire_version_without_legacy_chip() {
        let mut env = Envelope::new([0; 6], 1, SourceKind::EspVendor, 0);
        env.version = wire::WIRE_VERSION + 1;
        let info = SessionInfo::new(Chip::Esp32, [0, 13, 0], 0, None);
        let f = encode(&env, &Body::Session(info));
        let err = FrameDecoder::new(None).decode(&f).unwrap_err();
        assert!(matches!(err, DecodeError::Wire(wire::DecodeError::UnsupportedVersion(2))));
    }
}
