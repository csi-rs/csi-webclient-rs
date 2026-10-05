//! Fallback decoder for the pre-0.12 `serialized` format (esp-csi-rs 0.8 – 0.11).
//!
//! Before 0.12 each serialized record was `postcard::to_slice_cobs(&CSIDataPacket)`, and the
//! struct's layout depended on the chip: postcard is not self-describing, so the host has to know
//! the chip to pick the right mirror. The mirrors below match esp-csi-rs 0.8.0 through 0.11.x field
//! for field (the struct did not change in that range). They are frozen: new firmware speaks the
//! [`crate::wire`] format instead.
//!
//! A decoded packet is converted into the wire types ([`CsiFrame`]) with the same field mapping the
//! 0.12 firmware applies on the device, so everything downstream handles one shape.

use heapless::Vec as HVec;
use serde::{Deserialize, Serialize};

use crate::wire::{
    Bandwidth, Chip, CsiFrame, CsiPayload, LayoutId, MAX_CSI_BYTES, PpduFormat, RxMeta, Secondary,
    Stimulus, VendorRx,
};

/// Which on-device `CSIDataPacket` layout a connected chip produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipVariant {
    /// esp32, esp32c3, esp32s3 — the full radio-metadata layout ([`PacketA`]).
    Esp32Family,
    /// esp32c5 — the reduced layout without the c6-only fields ([`PacketBc5`]).
    Esp32c5,
    /// esp32c6 — the reduced layout plus `sigb_len`/`cur_single_mpdu`/`rxmatch0`.
    Esp32c6,
}

impl ChipVariant {
    /// Map a firmware `chip=` string (case-insensitive) to its wire layout.
    ///
    /// Returns `None` for unrecognized chips so the caller can refuse to decode
    /// rather than guess a layout.
    pub fn from_chip_str(chip: &str) -> Option<Self> {
        match chip.trim().to_ascii_lowercase().as_str() {
            "esp32" | "esp32c3" | "esp32s3" | "esp32s2" => Some(Self::Esp32Family),
            "esp32c5" => Some(Self::Esp32c5),
            "esp32c6" => Some(Self::Esp32c6),
            _ => None,
        }
    }
}

/// Optional NTP-derived calendar timestamp the firmware may attach to a packet.
///
/// Mirror of `esp_csi_rs::time::DateTime` (all fields `u64`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateTime {
    pub year: u64,
    pub month: u64,
    pub day: u64,
    pub hour: u64,
    pub minute: u64,
    pub second: u64,
    pub millisecond: u64,
}

/// Compact CSI data-format descriptor.
///
/// Mirror of `esp_csi_rs::csi::RxCSIFmt` — **variant order is the wire encoding**
/// (postcard encodes the discriminant as a varint of the declaration index), so
/// do not reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RxCsiFmt {
    Bw20,
    HtBw20,
    HtBw20Stbc,
    SecbBw20,
    SecbHtBw20,
    SecbHtBw20Stbc,
    SecbHtBw40,
    SecbHtBw40Stbc,
    SecaBw20,
    SecaHtBw20,
    SecaHtBw20Stbc,
    SecaHtBw40,
    SecaHtBw40Stbc,
    /// VHT 20 MHz (`cur_bb_format == 3` on C5/C6).
    VhtBw20,
    /// Any format the core library does not name (discriminant 14). Higher
    /// numeric `cur_bb_format` values decode here with the raw value preserved
    /// in the `cur_bb_format` column; a [`crate::profile::ClientProfile`] can
    /// relabel them for the export.
    Undefined,
}

impl RxCsiFmt {
    /// Stable identifier for the Parquet `data_format` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bw20 => "Bw20",
            Self::HtBw20 => "HtBw20",
            Self::HtBw20Stbc => "HtBw20Stbc",
            Self::SecbBw20 => "SecbBw20",
            Self::SecbHtBw20 => "SecbHtBw20",
            Self::SecbHtBw20Stbc => "SecbHtBw20Stbc",
            Self::SecbHtBw40 => "SecbHtBw40",
            Self::SecbHtBw40Stbc => "SecbHtBw40Stbc",
            Self::SecaBw20 => "SecaBw20",
            Self::SecaHtBw20 => "SecaHtBw20",
            Self::SecaHtBw20Stbc => "SecaHtBw20Stbc",
            Self::SecaHtBw40 => "SecaHtBw40",
            Self::SecaHtBw40Stbc => "SecaHtBw40Stbc",
            Self::VhtBw20 => "VhtBw20",
            Self::Undefined => "Undefined",
        }
    }
}

/// esp32 / esp32c3 / esp32s3 layout — mirror of `CSIDataPacket`
/// (`#[cfg(not(any(esp32c5, esp32c6)))]`). Field order is the wire order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketA {
    pub mac: [u8; 6],
    pub rssi: i32,
    pub timestamp: u32,
    pub rate: u32,
    pub sgi: u32,
    pub secondary_channel: u32,
    pub channel: u32,
    pub bandwidth: u32,
    pub antenna: u32,
    pub sig_mode: u32,
    pub mcs: u32,
    pub smoothing: u32,
    pub not_sounding: u32,
    pub aggregation: u32,
    pub stbc: u32,
    pub fec_coding: u32,
    pub ampdu_cnt: u32,
    pub noise_floor: i32,
    pub rx_state: u32,
    pub sig_len: u32,
    pub date_time: Option<DateTime>,
    pub sequence_number: u16,
    pub data_format: RxCsiFmt,
    pub csi_data_len: u16,
    pub csi_data: Vec<i8>,
}

/// esp32c5 layout — mirror of the `#[cfg(any(esp32c5, esp32c6))]` `CSIDataPacket`
/// **without** the `#[cfg(feature = "esp32c6")]` fields. Field order is the wire
/// order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketBc5 {
    pub mac: [u8; 6],
    pub rssi: i32,
    pub timestamp: u32,
    pub rate: u32,
    pub noise_floor: i32,
    pub sig_len: u32,
    pub rx_state: u32,
    pub dump_len: u32,
    pub cur_bb_format: u32,
    pub rx_channel_estimate_info_vld: u32,
    pub rx_channel_estimate_len: u32,
    pub second: u32,
    pub channel: u32,
    pub is_group: u32,
    pub rxend_state: u32,
    pub rxmatch3: u32,
    pub rxmatch2: u32,
    pub rxmatch1: u32,
    pub date_time: Option<DateTime>,
    pub sequence_number: u16,
    pub csi_data_len: u16,
    pub data_format: RxCsiFmt,
    pub csi_data: Vec<i8>,
}

/// esp32c6 layout — the c5 layout plus the three `#[cfg(feature = "esp32c6")]`
/// fields (`sigb_len`, `cur_single_mpdu`, `rxmatch0`) at their declared
/// positions. Field order is the wire order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketBc6 {
    pub mac: [u8; 6],
    pub rssi: i32,
    pub timestamp: u32,
    pub rate: u32,
    pub noise_floor: i32,
    pub sig_len: u32,
    pub rx_state: u32,
    pub dump_len: u32,
    pub sigb_len: u32,
    pub cur_single_mpdu: u32,
    pub cur_bb_format: u32,
    pub rx_channel_estimate_info_vld: u32,
    pub rx_channel_estimate_len: u32,
    pub second: u32,
    pub channel: u32,
    pub is_group: u32,
    pub rxend_state: u32,
    pub rxmatch3: u32,
    pub rxmatch2: u32,
    pub rxmatch1: u32,
    pub rxmatch0: u32,
    pub date_time: Option<DateTime>,
    pub sequence_number: u16,
    pub csi_data_len: u16,
    pub data_format: RxCsiFmt,
    pub csi_data: Vec<i8>,
}

/// Failure decoding a pre-0.12 serialized CSI frame.
pub type LegacyError = postcard::Error;

impl ChipVariant {
    /// The wire layout family of `chip`.
    pub fn from_chip(chip: Chip) -> Self {
        match chip {
            Chip::Esp32C5 => Self::Esp32c5,
            Chip::Esp32C6 => Self::Esp32c6,
            Chip::Esp32 | Chip::Esp32S2 | Chip::Esp32S3 | Chip::Esp32C3 => Self::Esp32Family,
        }
    }
}

/// Map a firmware `chip=` string (case-insensitive) to a [`Chip`]. `None` if unrecognised.
pub fn chip_from_str(chip: &str) -> Option<Chip> {
    match chip.trim().to_ascii_lowercase().as_str() {
        "esp32" => Some(Chip::Esp32),
        "esp32s2" => Some(Chip::Esp32S2),
        "esp32s3" => Some(Chip::Esp32S3),
        "esp32c3" => Some(Chip::Esp32C3),
        "esp32c5" => Some(Chip::Esp32C5),
        "esp32c6" => Some(Chip::Esp32C6),
        _ => None,
    }
}

/// A decoded pre-0.12 packet, converted to the wire types.
#[derive(Debug, Clone)]
pub struct LegacyFrame {
    /// The measurement, mapped as the 0.12 firmware maps it.
    pub frame: CsiFrame,
    /// The packet's own `data_format`, kept for the export's `data_format` column.
    pub data_format: RxCsiFmt,
}

/// Decode one COBS-framed pre-0.12 record laid out for `chip`.
///
/// `frame` is the WebSocket payload (the COBS body, with or without the trailing `\0`). The input
/// is copied because COBS decoding works in place.
pub fn decode(frame: &[u8], chip: Chip) -> Result<LegacyFrame, LegacyError> {
    let mut owned = frame.to_vec();
    let out = match ChipVariant::from_chip(chip) {
        ChipVariant::Esp32Family => {
            let (p, _) = postcard::take_from_bytes_cobs::<PacketA>(&mut owned)?;
            LegacyFrame { data_format: p.data_format, frame: classic_frame(p, chip)? }
        }
        ChipVariant::Esp32c5 => {
            let (p, _) = postcard::take_from_bytes_cobs::<PacketBc5>(&mut owned)?;
            let fields = HeFields {
                mac: p.mac,
                rssi: p.rssi,
                timestamp: p.timestamp,
                rate: p.rate,
                noise_floor: p.noise_floor,
                sig_len: p.sig_len,
                rx_state: p.rx_state,
                dump_len: p.dump_len,
                cur_bb_format: p.cur_bb_format,
                estimate_valid: p.rx_channel_estimate_info_vld,
                estimate_len: p.rx_channel_estimate_len,
                second: p.second,
                channel: p.channel,
                is_group: p.is_group,
                rxend_state: p.rxend_state,
                rxmatch: (p.rxmatch1 & 1) << 1 | (p.rxmatch2 & 1) << 2 | (p.rxmatch3 & 1) << 3,
                sigb_len: 0,
                single_mpdu: 0,
                sequence_number: p.sequence_number,
            };
            LegacyFrame { data_format: p.data_format, frame: he_frame(fields, chip, p.csi_data)? }
        }
        ChipVariant::Esp32c6 => {
            let (p, _) = postcard::take_from_bytes_cobs::<PacketBc6>(&mut owned)?;
            let fields = HeFields {
                mac: p.mac,
                rssi: p.rssi,
                timestamp: p.timestamp,
                rate: p.rate,
                noise_floor: p.noise_floor,
                sig_len: p.sig_len,
                rx_state: p.rx_state,
                dump_len: p.dump_len,
                cur_bb_format: p.cur_bb_format,
                estimate_valid: p.rx_channel_estimate_info_vld,
                estimate_len: p.rx_channel_estimate_len,
                second: p.second,
                channel: p.channel,
                is_group: p.is_group,
                rxend_state: p.rxend_state,
                rxmatch: (p.rxmatch0 & 1)
                    | (p.rxmatch1 & 1) << 1
                    | (p.rxmatch2 & 1) << 2
                    | (p.rxmatch3 & 1) << 3,
                sigb_len: p.sigb_len,
                single_mpdu: p.cur_single_mpdu,
                sequence_number: p.sequence_number,
            };
            LegacyFrame { data_format: p.data_format, frame: he_frame(fields, chip, p.csi_data)? }
        }
    };
    Ok(out)
}

fn secondary_of(code: u32) -> Secondary {
    match code {
        1 => Secondary::Above,
        2 => Secondary::Below,
        _ => Secondary::None,
    }
}

fn raw_bytes(data: Vec<i8>) -> Result<HVec<i8, MAX_CSI_BYTES>, LegacyError> {
    HVec::from_slice(&data).map_err(|_| postcard::Error::DeserializeBadEncoding)
}

/// Classic MAC (ESP32, S2/S3, C3): the mapping of esp-csi-rs 0.12 `csi::esp::rx_meta`.
fn classic_frame(p: PacketA, chip: Chip) -> Result<CsiFrame, LegacyError> {
    let sig_mode = p.sig_mode as u8;
    let rate = p.rate as u8;
    let forty = p.bandwidth == 1;
    let stbc = p.stbc != 0;
    let mcs = p.mcs as u8;
    let ppdu = match sig_mode {
        0 if rate < 8 => PpduFormat::Dsss,
        0 => PpduFormat::NonHt,
        1 => PpduFormat::Ht,
        3 => PpduFormat::Vht,
        _ => PpduFormat::Unknown,
    };
    let ht_like = matches!(ppdu, PpduFormat::Ht | PpduFormat::Vht);
    let layout = LayoutId::classify_classic(
        p.secondary_channel as u8,
        sig_mode,
        forty,
        stbc,
        p.csi_data.len(),
    );
    let meta = RxMeta {
        timestamp_us: u64::from(p.timestamp),
        rssi: p.rssi as i8,
        noise_floor: p.noise_floor as i8,
        channel: p.channel as u8,
        secondary: secondary_of(p.secondary_channel),
        bandwidth: Some(if forty { Bandwidth::Mhz40 } else { Bandwidth::Mhz20 }),
        ppdu,
        mcs: ht_like.then_some(mcs),
        stbc: Some(stbc),
        sgi: Some(p.sgi != 0),
        n_rx: 1,
        n_ss: Some(if ppdu == PpduFormat::Ht { mcs / 8 + 1 } else { 1 }),
        antenna: Some(p.antenna as u8),
        sig_len: p.sig_len as u16,
        rx_state: p.rx_state as u8,
        not_sounding: Some(p.not_sounding != 0),
        aggregation: Some(p.aggregation != 0),
        frame_seq: Some(p.sequence_number),
        vendor: VendorRx::EspClassic {
            rate,
            sig_mode,
            smoothing: p.smoothing != 0,
            fec_ldpc: p.fec_coding != 0,
            ampdu_cnt: p.ampdu_cnt as u8,
        },
    };
    Ok(CsiFrame {
        meta,
        stimulus: Stimulus::Ambient { ta: p.mac },
        header: None,
        payload: CsiPayload::EspRaw {
            chip,
            layout,
            // Pre-0.12 packets did not carry the flag.
            first_word_invalid: false,
            bytes: raw_bytes(p.csi_data)?,
        },
    })
}

/// The C5/C6 fields both HE-generation layouts share.
struct HeFields {
    mac: [u8; 6],
    rssi: i32,
    timestamp: u32,
    rate: u32,
    noise_floor: i32,
    sig_len: u32,
    rx_state: u32,
    dump_len: u32,
    cur_bb_format: u32,
    estimate_valid: u32,
    estimate_len: u32,
    /// Despite its name, the secondary-channel code (0 none, 1 above, 2 below).
    second: u32,
    channel: u32,
    is_group: u32,
    rxend_state: u32,
    rxmatch: u32,
    sigb_len: u32,
    single_mpdu: u32,
    sequence_number: u16,
}

/// 802.11ax-generation MAC (C5, C6): the mapping of esp-csi-rs 0.12 `csi::esp::rx_meta`.
fn he_frame(p: HeFields, chip: Chip, data: Vec<i8>) -> Result<CsiFrame, LegacyError> {
    let bb = p.cur_bb_format as u8;
    let ppdu = ppdu_of_bb_format(bb);
    let secondary = secondary_of(p.second);
    let layout = if chip == Chip::Esp32C5 {
        LayoutId::classify_c5(bb, p.second as u8, data.len())
    } else {
        LayoutId::Unknown
    };
    let (bandwidth, stbc) = match layout {
        LayoutId::C5Ht40 => (Some(Bandwidth::Mhz40), Some(false)),
        LayoutId::C5Ht40Stbc => (Some(Bandwidth::Mhz40), Some(true)),
        LayoutId::C5Ht20NoneStbc | LayoutId::C5Ht20BelowStbc | LayoutId::C5Ht20AboveStbc => {
            (Some(Bandwidth::Mhz20), Some(true))
        }
        LayoutId::Unknown => match ppdu {
            PpduFormat::HeSu | PpduFormat::HeMu | PpduFormat::HeErSu | PpduFormat::HeTb => {
                (Some(Bandwidth::Mhz20), None)
            }
            _ if secondary == Secondary::None => (Some(Bandwidth::Mhz20), None),
            _ => (None, None),
        },
        LayoutId::C5LltfNone | LayoutId::C5LltfBelow | LayoutId::C5LltfAbove => {
            (Some(Bandwidth::Mhz20), None)
        }
        _ => (Some(Bandwidth::Mhz20), Some(false)),
    };
    let meta = RxMeta {
        timestamp_us: u64::from(p.timestamp),
        rssi: p.rssi as i8,
        noise_floor: p.noise_floor as i8,
        channel: p.channel as u8,
        secondary,
        bandwidth,
        ppdu,
        mcs: None,
        stbc,
        sgi: None,
        n_rx: 1,
        n_ss: None,
        antenna: None,
        sig_len: p.sig_len as u16,
        rx_state: p.rx_state as u8,
        not_sounding: None,
        aggregation: None,
        frame_seq: Some(p.sequence_number),
        vendor: VendorRx::EspHe {
            rate: p.rate as u8,
            cur_bb_format: bb,
            estimate_valid: p.estimate_valid != 0,
            estimate_len: p.estimate_len as u16,
            dump_len: p.dump_len as u16,
            is_group: p.is_group != 0,
            rxend_state: p.rxend_state as u8,
            rxmatch: p.rxmatch as u8,
            // Not in the pre-0.12 packet.
            he_siga1: 0,
            he_siga2: 0,
            sigb_len: p.sigb_len as u8,
            single_mpdu: p.single_mpdu != 0,
        },
    };
    Ok(CsiFrame {
        meta,
        stimulus: Stimulus::Ambient { ta: p.mac },
        header: None,
        payload: CsiPayload::EspRaw {
            chip,
            layout,
            first_word_invalid: false,
            bytes: raw_bytes(data)?,
        },
    })
}

/// `wifi_rx_bb_format_t` to [`PpduFormat`], as esp-csi-rs 0.12 maps it.
pub fn ppdu_of_bb_format(bb: u8) -> PpduFormat {
    match bb {
        0 => PpduFormat::Dsss,
        1 => PpduFormat::NonHt,
        2 => PpduFormat::Ht,
        3 => PpduFormat::Vht,
        4 => PpduFormat::HeSu,
        5 => PpduFormat::HeMu,
        6 => PpduFormat::HeErSu,
        7 => PpduFormat::HeTb,
        11 => PpduFormat::VhtMu,
        _ => PpduFormat::Unknown,
    }
}

/// The 0.11 `data_format` name a 0.12 frame would have carried, so the export's `data_format`
/// column keeps its vocabulary across firmware versions.
pub fn data_format_of(meta: &RxMeta) -> RxCsiFmt {
    let forty = meta.bandwidth == Some(Bandwidth::Mhz40);
    let stbc = meta.stbc.unwrap_or(false);
    use RxCsiFmt::*;
    match (meta.ppdu, meta.secondary, forty, stbc) {
        (PpduFormat::Vht, _, false, _) => VhtBw20,
        (PpduFormat::Dsss | PpduFormat::NonHt, Secondary::None, _, _) => Bw20,
        (PpduFormat::Dsss | PpduFormat::NonHt, Secondary::Below, _, _) => SecbBw20,
        (PpduFormat::Dsss | PpduFormat::NonHt, Secondary::Above, _, _) => SecaBw20,
        (PpduFormat::Ht, Secondary::None, false, false) => HtBw20,
        (PpduFormat::Ht, Secondary::None, false, true) => HtBw20Stbc,
        (PpduFormat::Ht, Secondary::Below, false, false) => SecbHtBw20,
        (PpduFormat::Ht, Secondary::Below, false, true) => SecbHtBw20Stbc,
        (PpduFormat::Ht, Secondary::Below, true, false) => SecbHtBw40,
        (PpduFormat::Ht, Secondary::Below, true, true) => SecbHtBw40Stbc,
        (PpduFormat::Ht, Secondary::Above, false, false) => SecaHtBw20,
        (PpduFormat::Ht, Secondary::Above, false, true) => SecaHtBw20Stbc,
        (PpduFormat::Ht, Secondary::Above, true, false) => SecaHtBw40,
        (PpduFormat::Ht, Secondary::Above, true, true) => SecaHtBw40Stbc,
        _ => Undefined,
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip a `PacketA` through postcard+COBS exactly as the firmware
    /// emits it (`to_slice_cobs`), then decode it back. Guards against wire
    /// drift in field order/types for the esp32-family layout.
    #[test]
    fn decode_packet_a_roundtrip() {
        let pkt = PacketA {
            mac: [0xde, 0xad, 0xbe, 0xef, 0x00, 0x01],
            rssi: -42,
            timestamp: 123_456,
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
            noise_floor: -96,
            rx_state: 0,
            sig_len: 128,
            date_time: Some(DateTime {
                year: 2026,
                month: 6,
                day: 22,
                hour: 12,
                minute: 30,
                second: 15,
                millisecond: 250,
            }),
            sequence_number: 4242,
            data_format: RxCsiFmt::HtBw20,
            csi_data_len: 4,
            csi_data: vec![1, -2, 3, -4],
        };

        // Mirror the firmware: postcard-serialize then COBS-frame, then strip
        // the trailing `\0` the server removes before broadcasting.
        let mut buf = vec![0u8; 1024];
        let cobs = postcard::to_slice_cobs(&pkt, &mut buf).unwrap();
        let body = cobs.strip_suffix(&[0]).unwrap_or(cobs);

        let out = decode(body, Chip::Esp32).unwrap();
        let m = &out.frame.meta;
        assert_eq!(out.frame.transmitter(), Some(pkt.mac));
        assert_eq!(m.rssi, -42);
        assert_eq!(m.channel, 6);
        assert_eq!(m.mcs, Some(7));
        assert_eq!(m.noise_floor, -96);
        assert_eq!(m.frame_seq, Some(4242));
        assert_eq!(m.ppdu, PpduFormat::Ht);
        assert_eq!(m.bandwidth, Some(Bandwidth::Mhz20));
        assert_eq!(out.data_format, RxCsiFmt::HtBw20);
        assert_eq!(data_format_of(m), RxCsiFmt::HtBw20);
        assert!(matches!(m.vendor, VendorRx::EspClassic { rate: 11, sig_mode: 1, .. }));
        let CsiPayload::EspRaw { bytes, layout, .. } = &out.frame.payload else { panic!() };
        assert_eq!(bytes.as_slice(), &[1, -2, 3, -4]);
        // Four bytes match no table entry.
        assert_eq!(*layout, LayoutId::Unknown);
    }

    #[test]
    fn decode_packet_bc6_roundtrip() {
        let pkt = PacketBc6 {
            mac: [1, 2, 3, 4, 5, 6],
            rssi: -55,
            timestamp: 9,
            rate: 1,
            noise_floor: -90,
            sig_len: 64,
            rx_state: 0,
            dump_len: 100,
            sigb_len: 7,
            cur_single_mpdu: 1,
            cur_bb_format: 2,
            rx_channel_estimate_info_vld: 1,
            rx_channel_estimate_len: 64,
            second: 1,
            channel: 11,
            is_group: 0,
            rxend_state: 0,
            rxmatch3: 0,
            rxmatch2: 0,
            rxmatch1: 1,
            rxmatch0: 1,
            date_time: None,
            sequence_number: 7,
            csi_data_len: 2,
            data_format: RxCsiFmt::Undefined,
            csi_data: vec![-1, 1],
        };
        let mut buf = vec![0u8; 1024];
        let cobs = postcard::to_slice_cobs(&pkt, &mut buf).unwrap();
        let body = cobs.strip_suffix(&[0]).unwrap_or(cobs);

        let out = decode(body, Chip::Esp32C6).unwrap();
        let m = &out.frame.meta;
        assert_eq!(out.frame.transmitter(), Some([1, 2, 3, 4, 5, 6]));
        assert_eq!(m.sgi, None);
        assert_eq!(m.secondary, Secondary::Above);
        let VendorRx::EspHe { sigb_len, rxmatch, single_mpdu, cur_bb_format, .. } = m.vendor else {
            panic!("expected EspHe")
        };
        assert_eq!((sigb_len, rxmatch, single_mpdu, cur_bb_format), (7, 0b0011, true, 2));
        let CsiPayload::EspRaw { bytes, .. } = &out.frame.payload else { panic!() };
        assert_eq!(bytes.as_slice(), &[-1, 1]);
    }

    /// Wire-compat regression: a frame the firmware tags `Undefined` (the
    /// discriminant is now 14 after the higher-format variants were dropped)
    /// carrying a numeric `cur_bb_format` beyond the named set decodes cleanly,
    /// with the raw `cur_bb_format` preserved for a profile to relabel.
    #[test]
    fn undefined_format_preserves_numeric_cur_bb_format() {
        let pkt = PacketBc5 {
            mac: [1, 2, 3, 4, 5, 6],
            rssi: -50,
            timestamp: 1,
            rate: 1,
            noise_floor: -90,
            sig_len: 64,
            rx_state: 0,
            dump_len: 100,
            cur_bb_format: 4,
            rx_channel_estimate_info_vld: 1,
            rx_channel_estimate_len: 64,
            second: 0,
            channel: 36,
            is_group: 0,
            rxend_state: 0,
            rxmatch3: 0,
            rxmatch2: 0,
            rxmatch1: 0,
            date_time: None,
            sequence_number: 1,
            csi_data_len: 2,
            data_format: RxCsiFmt::Undefined,
            csi_data: vec![1, -1],
        };
        let mut buf = vec![0u8; 1024];
        let cobs = postcard::to_slice_cobs(&pkt, &mut buf).unwrap();
        let body = cobs.strip_suffix(&[0]).unwrap_or(cobs);

        let out = decode(body, Chip::Esp32C5).unwrap();
        assert_eq!(out.data_format, RxCsiFmt::Undefined);
        assert_eq!(out.frame.meta.ppdu, PpduFormat::HeSu);
        assert!(matches!(out.frame.meta.vendor, VendorRx::EspHe { cur_bb_format: 4, .. }));
    }

    /// A pre-0.12 C5 HE20 buffer is classified with the 0.12 table, so its subcarriers map.
    #[test]
    fn legacy_c5_he20_buffer_gets_a_layout() {
        let pkt = PacketBc5 {
            mac: [2, 2, 3, 4, 5, 6],
            rssi: -50,
            timestamp: 1,
            rate: 0,
            noise_floor: -90,
            sig_len: 64,
            rx_state: 0,
            dump_len: 490,
            cur_bb_format: 4,
            rx_channel_estimate_info_vld: 1,
            rx_channel_estimate_len: 490,
            second: 0,
            channel: 36,
            is_group: 0,
            rxend_state: 0,
            rxmatch3: 0,
            rxmatch2: 0,
            rxmatch1: 0,
            date_time: None,
            sequence_number: 1,
            csi_data_len: 490,
            data_format: RxCsiFmt::Undefined,
            csi_data: vec![0; 490],
        };
        let mut buf = vec![0u8; 2048];
        let cobs = postcard::to_slice_cobs(&pkt, &mut buf).unwrap();
        let out = decode(cobs, Chip::Esp32C5).unwrap();
        assert!(matches!(out.frame.payload, CsiPayload::EspRaw { layout: LayoutId::C5He20Su, .. }));
        assert_eq!(out.frame.meta.bandwidth, Some(Bandwidth::Mhz20));
    }

    #[test]
    fn chip_string_mapping() {
        assert_eq!(ChipVariant::from_chip_str("ESP32"), Some(ChipVariant::Esp32Family));
        assert_eq!(ChipVariant::from_chip_str("esp32c6"), Some(ChipVariant::Esp32c6));
        assert_eq!(ChipVariant::from_chip_str("esp32c5"), Some(ChipVariant::Esp32c5));
        assert_eq!(ChipVariant::from_chip_str("weird"), None);
        assert_eq!(chip_from_str(" ESP32C5 "), Some(Chip::Esp32C5));
        assert_eq!(chip_from_str("weird"), None);
    }
}
