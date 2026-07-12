use serde::{Deserialize, Serialize};

/// UI navigation tabs for the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Devices,
    Dashboard,
    Config,
    Control,
    Stream,
}

/// Wi-Fi operating modes accepted by `POST /api/devices/{id}/config/wifi`.
///
/// Serde names match [`Self::as_api_value`] so config snapshot files use the
/// same strings as the HTTP API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WiFiMode {
    Station,
    Sniffer,
    WifiAp,
    EspNowCentral,
    EspNowPeripheral,
    EspNowFastCollector,
    EspNowFastSource,
}

impl WiFiMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Station => "station",
            Self::Sniffer => "sniffer",
            Self::WifiAp => "wifi-ap",
            Self::EspNowCentral => "esp-now-central",
            Self::EspNowPeripheral => "esp-now-peripheral",
            Self::EspNowFastCollector => "esp-now-fast-collector",
            Self::EspNowFastSource => "esp-now-fast-source",
        }
    }

    /// Resolve a backend value back to a variant.
    pub fn from_api_value(value: &str) -> Option<Self> {
        match value {
            "station" => Some(Self::Station),
            "sniffer" => Some(Self::Sniffer),
            "wifi-ap" => Some(Self::WifiAp),
            "esp-now-central" => Some(Self::EspNowCentral),
            "esp-now-peripheral" => Some(Self::EspNowPeripheral),
            "esp-now-fast-collector" => Some(Self::EspNowFastCollector),
            "esp-now-fast-source" => Some(Self::EspNowFastSource),
            _ => None,
        }
    }

    /// True for all ESP-NOW operating modes (balanced and fast simplex).
    pub fn is_esp_now(self) -> bool {
        matches!(
            self,
            Self::EspNowCentral
                | Self::EspNowPeripheral
                | Self::EspNowFastCollector
                | Self::EspNowFastSource
        )
    }

    /// Requires `esp-csi-cli-rs` ≥ 0.7.0 on the device.
    pub fn requires_v07(self) -> bool {
        matches!(
            self,
            Self::WifiAp | Self::EspNowFastCollector | Self::EspNowFastSource
        )
    }

    /// Whether the channel is an optional pre-association *hint* rather than the
    /// operating channel.
    ///
    /// In station mode the firmware normally derives the channel from the
    /// associated AP; a supplied channel is forwarded as
    /// `WifiStationConfig::channel_hint` (a band-selection hint, meaningful on
    /// the ESP32-C5's 5 GHz band). Every mode accepts a `channel` field.
    pub fn channel_is_hint(self) -> bool {
        matches!(self, Self::Station)
    }
}

impl Default for WiFiMode {
    fn default() -> Self {
        Self::Station
    }
}

/// Collection role for the ESP32 firmware session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionMode {
    Collector,
    Listener,
}

impl CollectionMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Collector => "collector",
            Self::Listener => "listener",
        }
    }
}

impl Default for CollectionMode {
    fn default() -> Self {
        Self::Collector
    }
}

/// Forced ESP-NOW TX HT40 secondary-channel selection (`set-wifi --ht40`).
///
/// Only meaningful in ESP-NOW modes; ignored by the firmware otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ht40Mode {
    #[default]
    None,
    Above,
    Below,
}

impl Ht40Mode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Above => "above",
            Self::Below => "below",
        }
    }

    /// Resolve a backend value back to a variant (`off` aliases `none`).
    pub fn from_api_value(value: &str) -> Option<Self> {
        match value {
            "none" | "off" => Some(Self::None),
            "above" => Some(Self::Above),
            "below" => Some(Self::Below),
            _ => None,
        }
    }
}

/// Output routing mode for CSI frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    Stream,
    Dump,
    Both,
}

impl OutputMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::Dump => "dump",
            Self::Both => "both",
        }
    }
}

impl Default for OutputMode {
    fn default() -> Self {
        Self::Stream
    }
}

/// CSI delivery path accepted by `POST /api/devices/{id}/config/csi-delivery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsiDeliveryMode {
    Off,
    Callback,
    Async,
    /// Zero-copy fast-path; stored as a device flag, takes effect on next
    /// `start`. No CSI data is delivered or logged while active.
    Raw,
}

impl CsiDeliveryMode {
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Callback => "callback",
            Self::Async => "async",
            Self::Raw => "raw",
        }
    }
}

impl Default for CsiDeliveryMode {
    fn default() -> Self {
        Self::Async
    }
}

/// Wi-Fi PHY protocol applied at the start of each collection run
/// (`POST /api/devices/{id}/config/protocol`).
///
/// Default on the device is `lr` (Espressif Long-Range), which is
/// proprietary and won't associate with a standard AP — set `n`
/// explicitly for station mode.
///
/// [`Self::Ext`] is a generic escape hatch: a [`crate::profile::ClientProfile`]
/// can advertise additional protocol strings the core library does not name.
/// Those round-trip verbatim through [`Self::as_api_value`] /
/// [`Self::from_api_value`] and the JSON snapshot codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiProtocol {
    B,
    G,
    N,
    Lr,
    A,
    Ac,
    /// A profile-supplied protocol string (e.g. injected by a companion crate)
    /// carried through the client without the core library naming it.
    Ext(&'static str),
}

impl WifiProtocol {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::B => "b",
            Self::G => "g",
            Self::N => "n",
            Self::Lr => "lr",
            Self::A => "a",
            Self::Ac => "ac",
            Self::Ext(s) => s,
        }
    }

    /// Resolve a backend value back to a variant (case-insensitive).
    ///
    /// Unknown values become [`Self::Ext`] so a profile-supplied protocol
    /// round-trips even though the core library does not name it. The string is
    /// interned (leaked once) to obtain the `'static` lifetime; the set of
    /// distinct protocol strings a device reports is tiny and bounded.
    pub fn from_api_value(value: &str) -> Option<Self> {
        Some(match value.to_ascii_lowercase().as_str() {
            "b" => Self::B,
            "g" => Self::G,
            "n" => Self::N,
            "lr" => Self::Lr,
            "a" => Self::A,
            "ac" => Self::Ac,
            other => Self::Ext(intern(other)),
        })
    }
}

/// Intern a protocol string into a leaked `'static` slice. Called only from the
/// [`WifiProtocol::from_api_value`] fallback for profile-supplied protocols, so
/// the number of leaked strings is bounded by the distinct protocol names a
/// device ever reports.
fn intern(value: &str) -> &'static str {
    Box::leak(value.to_owned().into_boxed_str())
}

impl Serialize for WifiProtocol {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_api_value())
    }
}

impl<'de> Deserialize<'de> for WifiProtocol {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_api_value(&value).unwrap_or_default())
    }
}

impl Default for WifiProtocol {
    fn default() -> Self {
        Self::Lr
    }
}

/// PHY rate options accepted by `POST /api/devices/{id}/config/rate`.
///
/// Honored by all modes except `station` on the firmware side.
pub const PHY_RATES: &[&str] = &[
    "1m", "1m-l", "2m", "5m5", "5m5-l", "11m", "11m-l", "6m", "9m", "12m", "18m", "24m", "36m",
    "48m", "54m", "mcs0-lgi", "mcs1-lgi", "mcs2-lgi", "mcs3-lgi", "mcs4-lgi", "mcs5-lgi",
    "mcs6-lgi", "mcs7-lgi", "mcs0-sgi",
];

/// Editable Wi-Fi form values in the Config view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WiFiForm {
    pub mode: WiFiMode,
    pub sta_ssid: String,
    pub sta_password: String,
    pub ap_ssid: String,
    pub ap_password: String,
    pub ap_dhcp: bool,
    /// DHCP lease pool size in `wifi-ap` mode (1–8). With more than one
    /// lease the ICMP flood targets every associated station.
    pub ap_leases: u8,
    /// Synchronized burst flood in `wifi-ap` mode: every flood tick sends one
    /// frame to every active lease (time-aligned multi-receiver CSI) instead
    /// of round-robining one station per tick.
    pub ap_burst: bool,
    pub channel: String,
    /// ESP-NOW peer source-MAC filter (`aa:bb:cc:dd:ee:ff`); empty means
    /// clear back to automatic magic-prefix pairing. ESP-NOW modes only.
    pub peer_mac: String,
    /// Forced ESP-NOW TX HT40 secondary channel. ESP-NOW modes only.
    pub ht40: Ht40Mode,
}

impl Default for WiFiForm {
    fn default() -> Self {
        Self {
            mode: WiFiMode::Station,
            sta_ssid: String::new(),
            sta_password: String::new(),
            ap_ssid: "esp-csi-ap".to_owned(),
            ap_password: String::new(),
            ap_dhcp: true,
            ap_leases: 4,
            ap_burst: false,
            channel: String::new(),
            peer_mac: String::new(),
            ht40: Ht40Mode::None,
        }
    }
}

/// Pairing cookbook from esp-csi-cli-rs WEBSERVER.md (two-device setups).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingPreset {
    SoftApLab,
    EspNowFastSimplex,
    EspNowBalanced,
}

impl PairingPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::SoftApLab => "SoftAP lab pair",
            Self::EspNowFastSimplex => "ESP-NOW fast simplex",
            Self::EspNowBalanced => "ESP-NOW balanced",
        }
    }

    /// Whether this preset requires firmware ≥ 0.7.0 on both boards.
    pub fn requires_v07(self) -> bool {
        matches!(self, Self::SoftApLab | Self::EspNowFastSimplex)
    }
}

/// Editable traffic-generation form values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrafficForm {
    pub frequency_hz: String,
    /// Flood unsolicited echo replies instead of echo requests: strictly
    /// one-directional traffic (peer never answers at the IP level), stable
    /// offered rate — but this device captures no CSI back from replies.
    /// Only meaningful for WiFi AP/station modes with frequency > 0.
    pub unsolicited: bool,
}

impl Default for TrafficForm {
    fn default() -> Self {
        Self {
            frequency_hz: "100".to_owned(),
            unsolicited: false,
        }
    }
}

/// Editable CSI feature flags and numeric values.
///
/// Boolean fields use explicit on/off semantics matching `POST …/config/csi`
/// (`lltf`, `csi_legacy`, …). Defaults mirror firmware `reset-config`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CsiForm {
    // Classic (ESP32 / C3 / S3)
    pub lltf: bool,
    pub htltf: bool,
    pub stbc_htltf: bool,
    pub ltf_merge: bool,
    // HE (ESP32-C5 / C6)
    pub csi: bool,
    pub csi_legacy: bool,
    pub csi_ht20: bool,
    pub csi_ht40: bool,
    pub dump_ack: bool,
    pub csi_force_lltf: bool,
    pub csi_vht: bool,
    pub val_scale_cfg: String,
}

impl Default for CsiForm {
    fn default() -> Self {
        Self {
            lltf: true,
            htltf: true,
            stbc_htltf: true,
            ltf_merge: true,
            csi: true,
            csi_legacy: true,
            csi_ht20: true,
            csi_ht40: true,
            dump_ack: true,
            csi_force_lltf: true,
            csi_vht: true,
            val_scale_cfg: "2".to_owned(),
        }
    }
}

/// Editable PHY rate form value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PhyRateForm {
    pub rate: String,
}

impl Default for PhyRateForm {
    fn default() -> Self {
        Self {
            rate: "mcs0-lgi".to_owned(),
        }
    }
}

/// Editable IO tasks toggle form values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IoTasksForm {
    pub tx: bool,
    pub rx: bool,
}

impl Default for IoTasksForm {
    fn default() -> Self {
        Self { tx: true, rx: true }
    }
}

/// Editable CSI delivery form values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CsiDeliveryForm {
    pub mode: CsiDeliveryMode,
    pub logging: bool,
}

impl Default for CsiDeliveryForm {
    fn default() -> Self {
        Self {
            mode: CsiDeliveryMode::Async,
            logging: true,
        }
    }
}

/// All editable configuration form values for one device.
///
/// These mirror the per-device `config` sub-resources and are populated from
/// `GET /api/devices/{id}/config`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceForms {
    pub wifi: WiFiForm,
    pub traffic: TrafficForm,
    pub csi: CsiForm,
    pub collection_mode: CollectionMode,
    pub output_mode: OutputMode,
    pub protocol: WifiProtocol,
    pub phy_rate: PhyRateForm,
    pub io_tasks: IoTasksForm,
    pub csi_delivery: CsiDeliveryForm,
    pub start_duration_seconds: String,
}

/// On-disk JSON snapshot of one device's editable configuration.
///
/// This is the client-side form state (including Wi-Fi passwords, which the
/// server never returns), so a saved file can be replayed onto any device via
/// the full apply sequence. Missing fields fall back to form defaults, so
/// hand-trimmed files apply only the sections they contain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshotFile {
    /// Snapshot format marker for future migrations.
    #[serde(default = "snapshot_format_version")]
    pub format: u32,
    /// Device the snapshot was taken from (informational).
    #[serde(default)]
    pub device_id: Option<String>,
    /// Local wall-clock time the snapshot was saved (informational).
    #[serde(default)]
    pub saved_at: Option<String>,
    pub forms: DeviceForms,
}

/// Current [`ConfigSnapshotFile::format`] version.
fn snapshot_format_version() -> u32 {
    1
}

/// Lightweight frame metadata shown in the Stream tab.
#[derive(Debug, Clone, Default)]
pub struct FrameSummary {
    pub timestamp: String,
    pub length: usize,
    pub preview_hex: String,
}

/// Connection state to the webserver, derived from `GET /api/devices` results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServerStatus {
    /// No attempt has completed yet (initial state).
    #[default]
    Unknown,
    /// A connection attempt is in flight (e.g. Connect was just clicked).
    Connecting,
    /// The last `GET /api/devices` succeeded — the server is reachable.
    Connected,
    /// The last `GET /api/devices` failed (network error or HTTP error).
    Disconnected,
}

impl ServerStatus {
    /// Short human-readable label for display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Not connected",
            Self::Connecting => "Connecting…",
            Self::Connected => "Connected",
            Self::Disconnected => "Disconnected",
        }
    }
}

/// Ephemeral UI state that is not part of backend/device config.
#[derive(Debug, Clone)]
pub struct TransientUiState {
    pub active_tab: Tab,
    pub status_message: String,
    pub error_message: String,
    pub server_status: ServerStatus,
    /// Channel used by two-device pairing presets on the Devices tab.
    pub preset_channel: String,
}

impl Default for TransientUiState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Devices,
            status_message: "Ready".to_owned(),
            error_message: String::new(),
            server_status: ServerStatus::Unknown,
            preset_channel: "6".to_owned(),
        }
    }
}

/// Complete per-device state: identity, config forms, status, and stream.
///
/// Every attached device discovered via `GET /api/devices` owns one of these.
/// State is fully isolated per device, matching the server's per-device model.
#[derive(Debug, Clone)]
pub struct DeviceState {
    /// Stable device id used to build `/api/devices/{id}/...` paths.
    pub id: String,
    /// Stable board MAC when reported by the webserver (USB iSerialNumber).
    pub mac: Option<String>,
    // ---- live metadata from GET /api/devices ----
    pub port_path: Option<String>,
    pub baud_rate: Option<u32>,
    pub serial_connected: Option<bool>,
    pub collection_running: Option<bool>,
    pub firmware_verified: Option<bool>,
    /// Chip fault reported by the webserver (e.g. the ESP32-C5/C6 USB-JTAG
    /// reset-loop wedge), including the recovery action. `None` = healthy.
    pub fault: Option<String>,
    pub latest_info: Option<DeviceInfo>,
    // ---- editable config + cache ----
    pub forms: DeviceForms,
    pub latest_config: Option<DeviceConfig>,
    /// Guards against an infinite reset/fetch loop when an empty fetch
    /// auto-issues `config/reset` and the follow-up fetch is also empty.
    pub auto_resetting_cache: bool,
    // ---- stream ----
    pub ws_connected: bool,
    pub frames_received: u64,
    pub bytes_received: u64,
    pub recent_frames: Vec<FrameSummary>,
    pub auto_scroll_stream: bool,
    /// True once the one-time info/config/status fetch has been issued for
    /// this freshly-discovered device.
    pub details_loaded: bool,
    // ---- config save/load/copy ----
    /// Config snapshot file path in the Config tab; empty = auto-name on save.
    pub config_path: String,
    /// Device id selected in the "Copy from" picker on the Config tab.
    pub copy_source: String,
    // ---- local Parquet export ----
    /// True while this device's stream is being recorded to a Parquet file.
    pub recording: bool,
    /// Output path of the current (or most recent) recording.
    pub record_path: Option<String>,
    /// Frames written to the active recording.
    pub recorded_frames: u64,
    /// Frames the recorder could not decode (wire drift / truncation).
    pub record_decode_errors: u64,
}

impl DeviceState {
    /// Create a new device state with default forms.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            mac: None,
            port_path: None,
            baud_rate: None,
            serial_connected: None,
            collection_running: None,
            firmware_verified: None,
            fault: None,
            latest_info: None,
            forms: DeviceForms::default(),
            latest_config: None,
            auto_resetting_cache: false,
            ws_connected: false,
            frames_received: 0,
            bytes_received: 0,
            recent_frames: Vec::new(),
            auto_scroll_stream: true,
            details_loaded: false,
            config_path: String::new(),
            copy_source: String::new(),
            recording: false,
            record_path: None,
            recorded_frames: 0,
            record_decode_errors: 0,
        }
    }

    /// Refresh the live status fields from a `GET /api/devices` list entry.
    ///
    /// This keeps the dashboard/fleet view current on every discovery poll
    /// without per-device round-trips.
    pub fn apply_list_entry(&mut self, entry: &DeviceListEntry) {
        self.mac = entry.mac.clone();
        self.port_path = entry.port_path.clone();
        self.baud_rate = entry.baud_rate;
        self.serial_connected = entry.serial_connected;
        self.collection_running = entry.collection_running;
        self.firmware_verified = entry.firmware_verified;
        self.fault = entry.fault.clone();
        if let Some(info) = &entry.device_info {
            self.latest_info = Some(info.clone());
        }
    }

    /// Record one received frame and update stream counters/history.
    pub fn push_frame(&mut self, bytes: &[u8]) {
        self.frames_received = self.frames_received.saturating_add(1);
        self.bytes_received = self.bytes_received.saturating_add(bytes.len() as u64);

        let preview = bytes
            .iter()
            .take(24)
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        self.recent_frames.push(FrameSummary {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            length: bytes.len(),
            preview_hex: preview,
        });

        if self.recent_frames.len() > 300 {
            let drain_to = self.recent_frames.len() - 300;
            self.recent_frames.drain(0..drain_to);
        }
    }

    /// Clear stream frames and counters for this device.
    pub fn clear_frames(&mut self) {
        self.recent_frames.clear();
        self.frames_received = 0;
        self.bytes_received = 0;
    }

    /// Apply a `control/status` payload to this device's runtime status.
    pub fn apply_control_status(&mut self, status: ControlStatus) {
        self.serial_connected = status.serial_connected;
        self.collection_running = status.collection_running;
        self.port_path = status.port_path;
    }

    /// Apply server config payload into this device's form fields.
    ///
    /// Returns the number of fields that were actually applied; callers
    /// use a zero return to detect an empty server cache.
    pub fn apply_device_config(&mut self, config: DeviceConfig) -> usize {
        let mut applied = 0;
        let forms = &mut self.forms;

        if let Some(wifi) = config.wifi.as_ref() {
            if let Some(mode) = wifi.mode.as_deref() {
                if let Some(parsed) = WiFiMode::from_api_value(mode) {
                    forms.wifi.mode = parsed;
                    applied += 1;
                }
            }
            if let Some(channel) = wifi.channel {
                forms.wifi.channel = channel.to_string();
                applied += 1;
            }
            if let Some(ssid) = &wifi.sta_ssid {
                forms.wifi.sta_ssid = ssid.clone();
                applied += 1;
            }
            if let Some(ap_ssid) = &wifi.ap_ssid {
                forms.wifi.ap_ssid = ap_ssid.clone();
                applied += 1;
            }
            if let Some(ap_dhcp) = wifi.ap_dhcp {
                forms.wifi.ap_dhcp = ap_dhcp;
                applied += 1;
            }
            if let Some(ap_leases) = wifi.ap_leases {
                forms.wifi.ap_leases = ap_leases;
                applied += 1;
            }
            if let Some(ap_burst) = wifi.ap_burst {
                forms.wifi.ap_burst = ap_burst;
                applied += 1;
            }
            if let Some(peer_mac) = &wifi.peer_mac {
                // The server reports "auto" for the default pairing; surface
                // that as an empty field so re-submitting keeps it automatic.
                forms.wifi.peer_mac = if peer_mac == "auto" {
                    String::new()
                } else {
                    peer_mac.clone()
                };
                applied += 1;
            }
            if let Some(ht40) = wifi.ht40.as_deref() {
                if let Some(parsed) = Ht40Mode::from_api_value(ht40) {
                    forms.wifi.ht40 = parsed;
                    applied += 1;
                }
            }
        }

        if let Some(collection) = config.collection.as_ref() {
            if let Some(traffic_hz) = collection.traffic_hz {
                forms.traffic.frequency_hz = traffic_hz.to_string();
                applied += 1;
            }
            if let Some(unsolicited) = collection.unsolicited {
                forms.traffic.unsolicited = unsolicited;
                applied += 1;
            }
            if let Some(mode) = collection.mode.as_deref() {
                forms.collection_mode = if mode == "listener" {
                    CollectionMode::Listener
                } else {
                    CollectionMode::Collector
                };
                applied += 1;
            }
            if let Some(rate) = &collection.phy_rate {
                forms.phy_rate.rate = rate.clone();
                applied += 1;
            }
            if let Some(protocol) = collection.protocol.as_deref() {
                if let Some(parsed) = WifiProtocol::from_api_value(protocol) {
                    forms.protocol = parsed;
                    applied += 1;
                }
            }
            if let Some(tx) = collection.io_tx_enabled {
                forms.io_tasks.tx = tx;
                applied += 1;
            }
            if let Some(rx) = collection.io_rx_enabled {
                forms.io_tasks.rx = rx;
                applied += 1;
            }
        }

        if let Some(csi_cfg) = config.csi_config.as_ref() {
            if let Some(v) = csi_cfg.lltf_enabled {
                forms.csi.lltf = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.htltf_enabled {
                forms.csi.htltf = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.stbc_htltf_enabled {
                forms.csi.stbc_htltf = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.ltf_merge_enabled {
                forms.csi.ltf_merge = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi {
                forms.csi.csi = v != 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_legacy {
                forms.csi.csi_legacy = v != 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_ht20 {
                forms.csi.csi_ht20 = v != 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_ht40 {
                forms.csi.csi_ht40 = v != 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.dump_ack_enabled {
                forms.csi.dump_ack = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_force_lltf {
                forms.csi.csi_force_lltf = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_vht {
                forms.csi.csi_vht = v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.val_scale_cfg {
                forms.csi.val_scale_cfg = v.to_string();
                applied += 1;
            }
        }

        if let Some(mode) = config.csi_delivery_mode.as_deref() {
            forms.csi_delivery.mode = match mode {
                "off" => CsiDeliveryMode::Off,
                "callback" => CsiDeliveryMode::Callback,
                "raw" => CsiDeliveryMode::Raw,
                _ => CsiDeliveryMode::Async,
            };
            applied += 1;
        }
        if let Some(logging) = config.csi_logging_enabled {
            forms.csi_delivery.logging = logging;
            applied += 1;
        }

        self.latest_config = Some(config);
        applied
    }
}

/// High-level user actions queued by the UI for orchestration.
#[derive(Debug, Clone)]
pub enum UserIntent {
    /// Refresh the attached-device list (`GET /api/devices`).
    FetchDevices,
    /// Toggle whether a device is in the detail-tab selection set.
    ToggleDeviceSelection(String),
    /// Select every attached device for the detail tabs.
    SelectAllDevices,
    /// Clear the detail-tab device selection.
    ClearDeviceSelection,
    /// Start collection on every attached device.
    StartAllCollections { duration_seconds: String },
    /// Stop collection on every attached device.
    StopAllCollections,
    /// Start collection on every selected device (synchronized FDM start).
    StartSelectedCollections { duration_seconds: String },
    /// Stop collection on every selected device.
    StopSelectedCollections,
    /// Start Parquet recording on every selected device.
    StartSelectedRecording,
    /// Stop Parquet recording on every selected device.
    StopSelectedRecording,
    /// An action addressed to one specific device.
    Device { id: String, action: DeviceAction },
}

/// Per-device action; addressed to a device id by [`UserIntent::Device`].
#[derive(Debug, Clone)]
pub enum DeviceAction {
    FetchConfig,
    FetchInfo,
    FetchStatus,
    ResetConfig,
    SetWifi(WiFiForm),
    SetTraffic(TrafficForm),
    SetCsi(CsiForm),
    /// Apply a named full CSI acquisition preset (e.g. `default`). Additional
    /// preset names may be supplied by a [`crate::profile::ClientProfile`].
    SetCsiPreset(&'static str),
    SetCollectionMode(CollectionMode),
    SetOutputMode(OutputMode),
    SetProtocol(WifiProtocol),
    SetPhyRate(PhyRateForm),
    SetIoTasks(IoTasksForm),
    SetCsiDelivery(CsiDeliveryForm),
    StartCollection { duration_seconds: String },
    StopCollection,
    ShowStats,
    ResetDevice,
    ConnectWebSocket,
    DisconnectWebSocket,
    ClearFrames,
    /// Begin recording this device's incoming CSI stream to a Parquet file.
    StartRecording,
    /// Stop the active recording and finalize the Parquet file.
    StopRecording,
    /// Apply a two-device pairing cookbook (ordered config steps per board).
    ApplyPairingPreset {
        preset: PairingPreset,
        device_ids: [String; 2],
        channel: u8,
    },
    /// Save this device's form values to a JSON snapshot file.
    /// Empty path = auto-named file in the export directory.
    SaveConfigFile { path: String },
    /// Load a JSON snapshot file into this device's form values and apply
    /// every section to the device (same sequence as [`Self::ApplyFullConfig`]).
    LoadConfigFile { path: String },
    /// Push every config section from the current form values to the device
    /// as one ordered step sequence.
    ApplyFullConfig,
    /// Copy another device's form values onto this device and apply them.
    CopyConfigFrom { source_id: String },
}

/// One element of the `GET /api/devices` array.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceListEntry {
    pub id: String,
    pub mac: Option<String>,
    pub port_path: Option<String>,
    pub baud_rate: Option<u32>,
    pub serial_connected: Option<bool>,
    pub collection_running: Option<bool>,
    pub firmware_verified: Option<bool>,
    #[serde(default)]
    pub device_info: Option<DeviceInfo>,
    #[serde(default)]
    pub fault: Option<String>,
}

/// Wi-Fi section of `GET /api/devices/{id}/config`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceWifiConfig {
    pub mode: Option<String>,
    pub channel: Option<u16>,
    pub sta_ssid: Option<String>,
    pub ap_ssid: Option<String>,
    pub ap_dhcp: Option<bool>,
    pub ap_leases: Option<u8>,
    pub ap_burst: Option<bool>,
    pub peer_mac: Option<String>,
    pub ht40: Option<String>,
}

/// Collection section of `GET /api/devices/{id}/config`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCollectionConfig {
    pub mode: Option<String>,
    pub traffic_hz: Option<u64>,
    pub unsolicited: Option<bool>,
    pub phy_rate: Option<String>,
    pub protocol: Option<String>,
    pub io_tx_enabled: Option<bool>,
    pub io_rx_enabled: Option<bool>,
}

/// CSI section of `GET /api/devices/{id}/config`.
///
/// Mirrors firmware `show-config`: classic-chip booleans, HE-chip
/// `acquire_csi*` integers, plus read-only fields the device exposes
/// but does not accept via `POST /api/devices/{id}/config/csi`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCsiConfig {
    pub lltf_enabled: Option<bool>,
    pub htltf_enabled: Option<bool>,
    pub stbc_htltf_enabled: Option<bool>,
    pub ltf_merge_enabled: Option<bool>,
    pub channel_filter_enabled: Option<bool>,
    pub manual_scale: Option<bool>,
    pub shift: Option<i32>,
    pub dump_ack_enabled: Option<bool>,
    pub acquire_csi_force_lltf: Option<bool>,
    pub acquire_csi_vht: Option<bool>,
    pub acquire_csi: Option<u32>,
    pub acquire_csi_legacy: Option<u32>,
    pub acquire_csi_ht20: Option<u32>,
    pub acquire_csi_ht40: Option<u32>,
    pub val_scale_cfg: Option<u32>,
}

/// Cached server-side device configuration model.
///
/// Mirrors `GET /api/devices/{id}/config`. Sub-sections are `Option` so an
/// explicit `null` from the server (cache-not-yet-populated) deserializes as
/// `None` instead of erroring the whole payload out.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    #[serde(default)]
    pub wifi: Option<DeviceWifiConfig>,
    #[serde(default)]
    pub collection: Option<DeviceCollectionConfig>,
    #[serde(default)]
    pub csi_config: Option<DeviceCsiConfig>,
    pub csi_delivery_mode: Option<String>,
    pub csi_logging_enabled: Option<bool>,
}

/// Firmware identity from `GET /api/devices/{id}/info`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceInfo {
    pub banner_version: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub chip: Option<String>,
    pub mac: Option<String>,
    pub protocol: Option<u32>,
    #[serde(default)]
    pub features: Vec<String>,
}

impl DeviceInfo {
    /// True when firmware is new enough for v0.7.0 Wi-Fi modes.
    pub fn supports_v07_modes(&self) -> bool {
        firmware_version_at_least(&self.version, 0, 7, 0)
            || firmware_version_at_least(&self.banner_version, 0, 7, 0)
    }

    /// True when firmware understands `set-traffic --unsolicited` (≥ 0.7.0).
    /// Older firmware's CLI rejects the WHOLE set-traffic command on an
    /// unknown flag ("Error: Did not understand"), silently discarding the
    /// frequency too — so the client must not send the flag to old firmware.
    pub fn supports_unsolicited(&self) -> bool {
        firmware_version_at_least(&self.version, 0, 7, 0)
            || firmware_version_at_least(&self.banner_version, 0, 7, 0)
    }
}

impl DeviceState {
    /// True when cached firmware info supports v0.7.0 modes.
    pub fn supports_v07_modes(&self) -> bool {
        self.latest_info
            .as_ref()
            .is_some_and(DeviceInfo::supports_v07_modes)
    }

    /// True when cached firmware info supports `set-traffic --unsolicited`.
    pub fn supports_unsolicited(&self) -> bool {
        self.latest_info
            .as_ref()
            .is_some_and(DeviceInfo::supports_unsolicited)
    }
}

/// Parse `major.minor.patch` and test ≥ `(req_major, req_minor, req_patch)`.
fn firmware_version_at_least(
    version: &Option<String>,
    req_major: u64,
    req_minor: u64,
    req_patch: u64,
) -> bool {
    let Some(v) = version.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    let mut parts = v.split('.');
    let Some(Ok(major)) = parts.next().map(str::parse) else {
        return false;
    };
    let Some(Ok(minor)) = parts.next().map(str::parse) else {
        return false;
    };
    let patch: u64 = parts
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);
    (major, minor, patch) >= (req_major, req_minor, req_patch)
}

/// Runtime status payload from `GET /api/devices/{id}/control/status`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlStatus {
    pub serial_connected: Option<bool>,
    pub collection_running: Option<bool>,
    pub port_path: Option<String>,
}

/// Result of reconciling a fresh `GET /api/devices` payload into state.
#[derive(Debug, Clone, Default)]
pub struct ReconcileOutcome {
    /// Ids of devices that were newly discovered this poll.
    pub new_ids: Vec<String>,
    /// Ids of devices that disappeared since the last poll.
    pub removed_ids: Vec<String>,
    /// True when the device set (membership or selection) changed.
    pub changed: bool,
}

/// Full application state.
///
/// This is the single source of truth for all UI-visible data.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub server_host: String,
    pub server_port: String,
    /// Directory where locally-recorded Parquet exports are written.
    pub export_dir: String,
    pub devices: Vec<DeviceState>,
    /// Devices currently selected for the detail tabs. Multiple devices can be
    /// selected at once for side-by-side (FDM mesh) collection.
    pub selected_device_ids: Vec<String>,
    pub transient: TransientUiState,
    pub events: Vec<String>,
    intent_queue: Vec<UserIntent>,
}

impl AppState {
    /// Construct default state with localhost webserver settings.
    pub fn with_defaults() -> Self {
        let mut state = Self::default();
        state.server_host = "127.0.0.1".to_owned();
        state.server_port = "3000".to_owned();
        state.export_dir = ".".to_owned();
        state
    }

    /// Queue one user intent.
    pub fn push_intent(&mut self, intent: UserIntent) {
        self.intent_queue.push(intent);
    }

    /// Queue a per-device action addressed to `id`.
    pub fn push_device_action(&mut self, id: impl Into<String>, action: DeviceAction) {
        self.intent_queue.push(UserIntent::Device {
            id: id.into(),
            action,
        });
    }

    /// Drain queued intents in FIFO order.
    pub fn drain_intents(&mut self) -> Vec<UserIntent> {
        std::mem::take(&mut self.intent_queue)
    }

    /// Append one event line to the global event history.
    pub fn push_event(&mut self, message: impl Into<String>) {
        self.events.push(message.into());
        if self.events.len() > 300 {
            let drain_to = self.events.len() - 300;
            self.events.drain(0..drain_to);
        }
    }

    /// Indices of the currently selected devices, in `devices` order (so the
    /// detail tabs render columns in a stable order regardless of click order).
    pub fn selected_indices(&self) -> Vec<usize> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| self.selected_device_ids.iter().any(|id| id == &d.id))
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Whether the device with the given id is currently selected.
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected_device_ids.iter().any(|s| s == id)
    }

    /// Toggle membership of `id` in the selection set.
    pub fn toggle_selection(&mut self, id: String) {
        if let Some(pos) = self.selected_device_ids.iter().position(|s| s == &id) {
            self.selected_device_ids.remove(pos);
        } else {
            self.selected_device_ids.push(id);
        }
    }

    /// Index of the device with the given id.
    pub fn device_index_by_id(&self, id: &str) -> Option<usize> {
        self.devices.iter().position(|d| d.id == id)
    }

    /// Mutable reference to the device with the given id.
    pub fn device_mut_by_id(&mut self, id: &str) -> Option<&mut DeviceState> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    /// Build the HTTP base URL from host/port fields.
    pub fn base_http_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.server_host.trim(),
            self.server_port.trim()
        )
    }

    /// Build the per-device WebSocket stream URL.
    pub fn device_ws_url(&self, id: &str) -> String {
        format!(
            "ws://{}:{}/api/devices/{}/ws",
            self.server_host.trim(),
            self.server_port.trim(),
            id
        )
    }

    /// Reconcile a fresh `GET /api/devices` payload into per-device state.
    ///
    /// Adds new devices (default forms), refreshes live status on existing
    /// ones, drops devices that vanished, and auto-selects the first device
    /// when the current selection is empty or stale.
    pub fn reconcile_devices(&mut self, entries: Vec<DeviceListEntry>) -> ReconcileOutcome {
        let mut outcome = ReconcileOutcome::default();

        let incoming_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();

        outcome.removed_ids = self
            .devices
            .iter()
            .map(|d| d.id.clone())
            .filter(|id| !incoming_ids.contains(id))
            .collect();

        let before = self.devices.len();
        self.devices.retain(|d| incoming_ids.contains(&d.id));
        if self.devices.len() != before {
            outcome.changed = true;
        }

        for entry in &entries {
            if let Some(idx) = self.device_index_by_id(&entry.id) {
                self.devices[idx].apply_list_entry(entry);
            } else {
                let mut device = DeviceState::new(entry.id.clone());
                device.apply_list_entry(entry);
                self.devices.push(device);
                outcome.new_ids.push(entry.id.clone());
                outcome.changed = true;
            }
        }

        // Drop any selected ids whose device vanished.
        let before_selection = self.selected_device_ids.len();
        self.selected_device_ids
            .retain(|id| self.devices.iter().any(|d| &d.id == id));
        if self.selected_device_ids.len() != before_selection {
            outcome.changed = true;
        }
        // Seed the selection with the first device on a fresh start so the
        // detail tabs aren't empty when devices are present.
        if self.selected_device_ids.is_empty() {
            if let Some(first) = self.devices.first() {
                self.selected_device_ids.push(first.id.clone());
                outcome.changed = true;
            }
        }

        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_config_parses_full_nested_response() {
        let json = r#"{
            "wifi": { "mode": "sniffer", "channel": 6, "sta_ssid": "MyNetwork" },
            "collection": {
                "mode": "collector", "traffic_hz": 100, "unsolicited": true,
                "phy_rate": "mcs0-lgi",
                "protocol": "n", "io_tx_enabled": true, "io_rx_enabled": true
            },
            "csi_config": {
                "lltf_enabled": true, "htltf_enabled": true,
                "stbc_htltf_enabled": true, "ltf_merge_enabled": true,
                "val_scale_cfg": 2,
                "acquire_csi": 1, "acquire_csi_legacy": 0
            },
            "csi_delivery_mode": "async",
            "csi_logging_enabled": true
        }"#;
        let cfg: DeviceConfig = serde_json::from_str(json).expect("parse");
        let mut device = DeviceState::new("ttyUSB0");
        let applied = device.apply_device_config(cfg);
        assert!(applied > 0);
        assert_eq!(device.forms.wifi.mode, WiFiMode::Sniffer);
        assert_eq!(device.forms.wifi.channel, "6");
        assert_eq!(device.forms.traffic.frequency_hz, "100");
        assert!(device.forms.traffic.unsolicited);
        assert!(device.forms.csi.csi);
        assert!(!device.forms.csi.csi_legacy);
        assert_eq!(device.forms.protocol, WifiProtocol::N);
    }

    #[test]
    fn device_config_tolerates_null_sub_objects() {
        let json = r#"{ "wifi": null, "collection": null, "csi_config": null }"#;
        let cfg: DeviceConfig = serde_json::from_str(json).expect("parse null subobjects");
        let mut device = DeviceState::new("ttyUSB0");
        assert_eq!(device.apply_device_config(cfg), 0);
    }

    #[test]
    fn device_config_tolerates_missing_sub_objects() {
        let cfg: DeviceConfig = serde_json::from_str("{}").expect("parse empty");
        let mut device = DeviceState::new("ttyUSB0");
        assert_eq!(device.apply_device_config(cfg), 0);
    }

    #[test]
    fn config_snapshot_roundtrips_via_json() {
        let mut forms = DeviceForms::default();
        forms.wifi.mode = WiFiMode::EspNowCentral;
        forms.wifi.channel = "11".to_owned();
        forms.wifi.sta_password = "hunter22".to_owned();
        forms.traffic.frequency_hz = "500".to_owned();
        // Exercise the generic profile-supplied protocol escape hatch.
        forms.protocol = WifiProtocol::Ext("myproto");
        forms.collection_mode = CollectionMode::Listener;
        forms.output_mode = OutputMode::Both;
        forms.csi_delivery.mode = CsiDeliveryMode::Raw;
        forms.csi.csi_vht = false;

        let snapshot = ConfigSnapshotFile {
            format: 1,
            device_id: Some("dev-a".to_owned()),
            saved_at: None,
            forms,
        };
        let json = serde_json::to_string_pretty(&snapshot).expect("serialize");
        // Enum values on disk match the HTTP API strings.
        assert!(json.contains("\"esp-now-central\""));
        assert!(json.contains("\"myproto\""));
        assert!(json.contains("\"listener\""));

        let parsed: ConfigSnapshotFile = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed.forms.wifi.mode, WiFiMode::EspNowCentral);
        assert_eq!(parsed.forms.wifi.channel, "11");
        assert_eq!(parsed.forms.wifi.sta_password, "hunter22");
        assert_eq!(parsed.forms.traffic.frequency_hz, "500");
        assert_eq!(parsed.forms.protocol, WifiProtocol::Ext("myproto"));
        assert_eq!(parsed.forms.collection_mode, CollectionMode::Listener);
        assert_eq!(parsed.forms.output_mode, OutputMode::Both);
        assert_eq!(parsed.forms.csi_delivery.mode, CsiDeliveryMode::Raw);
        assert!(!parsed.forms.csi.csi_vht);
    }

    #[test]
    fn config_snapshot_fills_missing_fields_with_defaults() {
        // A hand-trimmed file with only a Wi-Fi mode: everything else falls
        // back to form defaults, and the format marker defaults to 1.
        let json = r#"{ "forms": { "wifi": { "mode": "sniffer" } } }"#;
        let parsed: ConfigSnapshotFile = serde_json::from_str(json).expect("parse partial");
        assert_eq!(parsed.format, 1);
        assert_eq!(parsed.forms.wifi.mode, WiFiMode::Sniffer);
        assert_eq!(parsed.forms.wifi.ap_ssid, "esp-csi-ap");
        assert_eq!(parsed.forms.traffic.frequency_hz, "100");
        assert_eq!(parsed.forms.phy_rate.rate, "mcs0-lgi");
    }

    #[test]
    fn wifi_mode_parses_v07_values() {
        assert_eq!(
            WiFiMode::from_api_value("wifi-ap"),
            Some(WiFiMode::WifiAp)
        );
        assert!(WiFiMode::EspNowFastCollector.is_esp_now());
        assert!(WiFiMode::WifiAp.requires_v07());
    }

    #[test]
    fn wifi_mode_station_channel_is_hint() {
        assert!(WiFiMode::Station.channel_is_hint());
        assert!(!WiFiMode::Sniffer.channel_is_hint());
        assert!(!WiFiMode::WifiAp.channel_is_hint());
        assert!(!WiFiMode::EspNowCentral.channel_is_hint());
    }

    #[test]
    fn device_config_applies_ap_fields() {
        let json = r#"{
            "wifi": {
                "mode": "wifi-ap",
                "channel": 6,
                "ap_ssid": "lab-ap",
                "ap_dhcp": false
            }
        }"#;
        let cfg: DeviceConfig = serde_json::from_str(json).expect("parse");
        let mut device = DeviceState::new("D0CF13E290E8");
        device.apply_device_config(cfg);
        assert_eq!(device.forms.wifi.mode, WiFiMode::WifiAp);
        assert_eq!(device.forms.wifi.ap_ssid, "lab-ap");
        assert!(!device.forms.wifi.ap_dhcp);
    }

    #[test]
    fn firmware_version_gating() {
        let info = DeviceInfo {
            version: Some("0.7.0".to_owned()),
            ..Default::default()
        };
        assert!(info.supports_v07_modes());
        let old = DeviceInfo {
            version: Some("0.6.0".to_owned()),
            ..Default::default()
        };
        assert!(!old.supports_v07_modes());

        // `--unsolicited` is understood from 0.7.0 (the current release); older
        // firmware rejects the whole set-traffic command on the unknown flag.
        let v06 = DeviceInfo {
            version: Some("0.6.0".to_owned()),
            ..Default::default()
        };
        assert!(!v06.supports_unsolicited());
        let v07 = DeviceInfo {
            version: Some("0.7.0".to_owned()),
            ..Default::default()
        };
        assert!(v07.supports_unsolicited());
    }

    #[test]
    fn device_list_parses_mac_field() {
        let json = r#"[
            {
                "id": "D0-CF-13-E2-90-E8",
                "mac": "D0:CF:13:E2:90:E8",
                "port_path": "/dev/ttyACM0",
                "serial_connected": true,
                "firmware_verified": true
            }
        ]"#;
        let entries: Vec<DeviceListEntry> = serde_json::from_str(json).expect("parse list");
        let mut state = AppState::with_defaults();
        state.reconcile_devices(entries);
        assert_eq!(state.devices[0].id, "D0-CF-13-E2-90-E8");
        assert_eq!(
            state.devices[0].mac.as_deref(),
            Some("D0:CF:13:E2:90:E8")
        );
    }

    #[test]
    fn device_list_parses_and_reconciles() {
        let json = r#"[
            {
                "id": "ttyUSB0", "port_path": "/dev/ttyUSB0", "baud_rate": 115200,
                "serial_connected": true, "collection_running": false,
                "firmware_verified": true,
                "device_info": { "name": "esp-csi-cli-rs", "chip": "esp32c6", "protocol": 1 },
                "fault": "USB-JTAG reset loop (rst:0x15 USB_UART_HPSYS) — replug"
            }
        ]"#;
        let entries: Vec<DeviceListEntry> = serde_json::from_str(json).expect("parse list");
        let mut state = AppState::with_defaults();
        let outcome = state.reconcile_devices(entries);
        assert!(outcome.changed);
        assert_eq!(outcome.new_ids, vec!["ttyUSB0".to_owned()]);
        assert_eq!(state.devices.len(), 1);
        assert_eq!(state.selected_device_ids, vec!["ttyUSB0".to_owned()]);
        let device = &state.devices[0];
        assert_eq!(device.serial_connected, Some(true));
        assert_eq!(device.latest_info.as_ref().unwrap().chip.as_deref(), Some("esp32c6"));
        assert!(device.fault.as_deref().unwrap().contains("USB-JTAG reset loop"));
    }

    #[test]
    fn reconcile_drops_vanished_and_reselects() {
        let mut state = AppState::with_defaults();
        state.reconcile_devices(vec![
            DeviceListEntry { id: "a".to_owned(), ..Default::default() },
            DeviceListEntry { id: "b".to_owned(), ..Default::default() },
        ]);
        assert_eq!(state.selected_device_ids, vec!["a".to_owned()]);

        // Device "a" unplugged; selection should fall back to "b".
        let outcome = state.reconcile_devices(vec![DeviceListEntry {
            id: "b".to_owned(),
            ..Default::default()
        }]);
        assert!(outcome.changed);
        assert_eq!(outcome.removed_ids, vec!["a".to_owned()]);
        assert_eq!(state.devices.len(), 1);
        assert_eq!(state.selected_device_ids, vec!["b".to_owned()]);
    }
}
