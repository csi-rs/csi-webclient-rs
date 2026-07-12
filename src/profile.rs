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

    /// Whether the CSI section should surface the HE-STBC numeric field.
    fn shows_he_stbc_field(&self) -> bool {
        false
    }
}

/// No-op profile: the open build ships this and adds nothing chip-specific.
pub struct StandardClientProfile;

impl ClientProfile for StandardClientProfile {}
