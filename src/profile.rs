//! Injectable client-behavior seam.
//!
//! [`ClientProfile`] is the extension point that lets a downstream binary add
//! chip- or standard-specific behavior (extra PHY protocols, CSI presets,
//! `data_format` labeling) without the open library naming any of it. The open
//! binary ships [`StandardClientProfile`], a no-op implementation; a private
//! companion crate can ship its own profile injected via
//! [`crate::app::CsiClientApp::with_profile`].

use crate::state::DeviceAction;
use eframe::egui;

/// Pluggable per-deployment behavior for the client.
///
/// Every method is defaulted so the open build (and any consumer that only
/// wants the standard behavior) can use [`StandardClientProfile`] directly.
pub trait ClientProfile {
    /// Additional PHY-protocol option strings appended to the core set in the
    /// protocol dropdown. Each is surfaced as [`crate::state::WifiProtocol::Ext`]
    /// and round-trips to the server verbatim.
    fn extra_protocols(&self) -> &[&'static str] {
        &[]
    }

    /// Additional Wi-Fi operating-mode strings appended to the core set in the
    /// mode dropdown. Each is surfaced as [`crate::state::WiFiMode::Ext`] and
    /// round-trips to the server verbatim.
    fn extra_wifi_modes(&self) -> &[&'static str] {
        &[]
    }

    /// Whether the given Wi-Fi mode (its API string) ignores the normal
    /// device-side *capture* configuration — CSI flags, CSI delivery, IO tasks,
    /// CSI output, PHY protocol, PHY rate, traffic, and CSI presets. When
    /// `true`, the Config view hides those sections because they are no-ops or
    /// conflict for that mode.
    fn hides_capture_config(&self, mode_api: &str) -> bool {
        let _ = mode_api;
        false
    }

    /// Whether the given Wi-Fi mode produces no CSI at all. When `true`, the
    /// client additionally hides CSI *consumers* — the output-mode selector,
    /// the live Stream view, recording, and the WebSocket connect controls.
    fn produces_no_csi(&self, mode_api: &str) -> bool {
        let _ = mode_api;
        false
    }

    /// Render any mode-specific Wi-Fi fields for `mode_api` into `wifi_extra`,
    /// which is merged verbatim into the `set-wifi` request body. This is how a
    /// profile surfaces parameters (and relabels reused ones) the core library
    /// does not name. Called in the Wi-Fi section after the mode picker.
    fn extra_wifi_fields(
        &self,
        ui: &mut egui::Ui,
        wifi_extra: &mut std::collections::BTreeMap<String, serde_json::Value>,
        mode_api: &str,
    ) {
        let _ = (ui, wifi_extra, mode_api);
    }

    /// Render any extra CSI-preset buttons at the end of the CSI section.
    ///
    /// Implementations push [`DeviceAction::SetCsiPreset`] (or any other action)
    /// into `actions`; the caller routes them to the selected device.
    fn extra_preset_buttons(&self, ui: &mut egui::Ui, actions: &mut Vec<DeviceAction>) {
        let _ = (ui, actions);
    }

    /// Map a numeric `cur_bb_format` to a stable `data_format` label for the
    /// Parquet export. `None` falls back to the decoded `RxCsiFmt::as_str()`.
    fn label_format(&self, cur_bb_format: u32) -> Option<&'static str> {
        let _ = cur_bb_format;
        None
    }

    /// Whether the CSI section should surface the numeric STBC field the
    /// ESP32-C5/C6 CSI config exposes.
    fn shows_stbc_numeric_field(&self) -> bool {
        false
    }
}

/// No-op profile: the open build ships this and adds nothing chip-specific.
pub struct StandardClientProfile;

impl ClientProfile for StandardClientProfile {}
