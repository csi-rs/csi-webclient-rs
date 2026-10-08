//! The vendored esp-csi-rs `wire` module (src/wire) round-trips its own frames and rejects a
//! frame from another wire version before touching the body.

use crate::wire::{
    self, Bandwidth, Body, Chip, CsiFrame, CsiPayload, DecodeError, Envelope, LayoutId, PpduFormat,
    RxMeta, Secondary, SessionInfo, SourceKind, Stimulus, VendorRx,
};

fn frame() -> CsiFrame {
    let meta = RxMeta {
        timestamp_us: 1 << 40,
        rssi: -60,
        noise_floor: -95,
        channel: 6,
        secondary: Secondary::Above,
        bandwidth: Some(Bandwidth::Mhz40),
        ppdu: PpduFormat::Ht,
        mcs: Some(7),
        stbc: Some(false),
        sgi: Some(true),
        n_rx: 1,
        n_ss: Some(1),
        antenna: Some(0),
        sig_len: 300,
        rx_state: 0,
        not_sounding: Some(true),
        aggregation: Some(false),
        frame_seq: Some(4095),
        vendor: VendorRx::EspClassic { rate: 0, sig_mode: 1, smoothing: true, fec_ldpc: false, ampdu_cnt: 0 },
    };
    let bytes = (0..384).map(|i| (i % 127) as i8).collect();
    CsiFrame::new(
        meta,
        Stimulus::Ambient { ta: [0x24, 1, 2, 3, 4, 5] },
        None,
        CsiPayload::EspRaw { chip: Chip::Esp32, layout: LayoutId::ClassicAboveHt40, first_word_invalid: false, bytes },
    )
}

#[test]
fn decodes_a_frame_encoded_by_the_vendored_types() {
    let env = Envelope::new([0x30, 0xae, 0xa4, 0, 0, 1], 0xdead_beef, SourceKind::EspVendor, 42);
    let body = Body::Csi(frame());
    let mut buf = vec![0u8; wire::MAX_ENCODED_LEN];
    let encoded = wire::encode_cobs(&env, &body, &mut buf).unwrap().to_vec();
    assert_eq!(encoded.last(), Some(&0));

    // With the delimiter, and without it (as csi-webserver forwards frames).
    for mut f in [encoded.clone(), encoded[..encoded.len() - 1].to_vec()] {
        let (e, b) = wire::decode_cobs(&mut f).unwrap();
        assert_eq!(e, env);
        assert_eq!(b, body);
    }

    let Body::Csi(f) = body else { unreachable!() };
    let CsiPayload::EspRaw { layout, bytes, .. } = &f.payload else { unreachable!() };
    assert_eq!(layout.byte_len(), Some(bytes.len()));
    assert_eq!(layout.indices().count(), bytes.len() / 2);
}

#[test]
fn rejects_a_wrong_version() {
    let mut env = Envelope::new([0; 6], 1, SourceKind::EspVendor, 0);
    env.version = wire::WIRE_VERSION + 1;
    let body = Body::Session(SessionInfo::new(Chip::Esp32C5, [0, 13, 0], 0, None));
    let mut buf = vec![0u8; wire::MAX_ENCODED_LEN];
    let mut encoded = wire::encode_cobs(&env, &body, &mut buf).unwrap().to_vec();
    assert_eq!(
        wire::decode_cobs(&mut encoded),
        Err(DecodeError::UnsupportedVersion(wire::WIRE_VERSION + 1))
    );
}
