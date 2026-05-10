use serde::{Deserialize, Serialize};

/// UI navigation tabs for the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Dashboard,
    Config,
    Control,
    Stream,
}

/// Wi-Fi operating modes accepted by `POST /api/config/wifi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiFiMode {
    Station,
    Sniffer,
    EspNowCentral,
    EspNowPeripheral,
}

impl WiFiMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Station => "station",
            Self::Sniffer => "sniffer",
            Self::EspNowCentral => "esp-now-central",
            Self::EspNowPeripheral => "esp-now-peripheral",
        }
    }

    /// Resolve a backend value back to a variant.
    pub fn from_api_value(value: &str) -> Option<Self> {
        match value {
            "station" => Some(Self::Station),
            "sniffer" => Some(Self::Sniffer),
            "esp-now-central" => Some(Self::EspNowCentral),
            "esp-now-peripheral" => Some(Self::EspNowPeripheral),
            _ => None,
        }
    }
}

impl Default for WiFiMode {
    fn default() -> Self {
        Self::Station
    }
}

/// Collection role for the ESP32 firmware session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Serial framing/log mode accepted by `POST /api/config/log-mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogMode {
    Text,
    ArrayList,
    Serialized,
    EspCsiTool,
}

impl LogMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::ArrayList => "array-list",
            Self::Serialized => "serialized",
            Self::EspCsiTool => "esp-csi-tool",
        }
    }
}

impl Default for LogMode {
    fn default() -> Self {
        Self::ArrayList
    }
}

/// Output routing mode for CSI frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// CSI delivery path accepted by `POST /api/config/csi-delivery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsiDeliveryMode {
    Off,
    Callback,
    Async,
}

impl CsiDeliveryMode {
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Callback => "callback",
            Self::Async => "async",
        }
    }
}

impl Default for CsiDeliveryMode {
    fn default() -> Self {
        Self::Async
    }
}

/// PHY rate options accepted by `POST /api/config/rate`.
///
/// Only honored by ESP-NOW central/peripheral modes on the firmware side.
pub const PHY_RATES: &[&str] = &[
    "1m", "1m-l", "2m", "5m5", "5m5-l", "11m", "11m-l", "6m", "9m", "12m", "18m", "24m", "36m",
    "48m", "54m", "mcs0-lgi", "mcs1-lgi", "mcs2-lgi", "mcs3-lgi", "mcs4-lgi", "mcs5-lgi",
    "mcs6-lgi", "mcs7-lgi", "mcs0-sgi",
];

/// Editable Wi-Fi form values in the Config view.
#[derive(Debug, Clone, Default)]
pub struct WiFiForm {
    pub mode: WiFiMode,
    pub sta_ssid: String,
    pub sta_password: String,
    pub channel: String,
}

/// Editable traffic configuration form values.
#[derive(Debug, Clone)]
pub struct TrafficForm {
    pub frequency_hz: String,
}

impl Default for TrafficForm {
    fn default() -> Self {
        Self {
            frequency_hz: "100".to_owned(),
        }
    }
}

/// Editable CSI feature flags and numeric values.
#[derive(Debug, Clone)]
pub struct CsiForm {
    pub disable_lltf: bool,
    pub disable_htltf: bool,
    pub disable_stbc_htltf: bool,
    pub disable_ltf_merge: bool,
    pub disable_csi: bool,
    pub disable_csi_legacy: bool,
    pub disable_csi_ht20: bool,
    pub disable_csi_ht40: bool,
    pub disable_csi_su: bool,
    pub disable_csi_mu: bool,
    pub disable_csi_dcm: bool,
    pub disable_csi_beamformed: bool,
    pub csi_he_stbc: String,
    pub val_scale_cfg: String,
}

impl Default for CsiForm {
    fn default() -> Self {
        Self {
            disable_lltf: false,
            disable_htltf: false,
            disable_stbc_htltf: false,
            disable_ltf_merge: false,
            disable_csi: false,
            disable_csi_legacy: false,
            disable_csi_ht20: false,
            disable_csi_ht40: false,
            disable_csi_su: false,
            disable_csi_mu: false,
            disable_csi_dcm: false,
            disable_csi_beamformed: false,
            csi_he_stbc: "2".to_owned(),
            val_scale_cfg: "2".to_owned(),
        }
    }
}

/// Editable PHY rate form value.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// User/session-level state persisted during app runtime.
#[derive(Debug, Clone, Default)]
pub struct PersistentState {
    pub server_host: String,
    pub server_port: String,
    pub wifi: WiFiForm,
    pub traffic: TrafficForm,
    pub csi: CsiForm,
    pub collection_mode: CollectionMode,
    pub log_mode: LogMode,
    pub output_mode: OutputMode,
    pub phy_rate: PhyRateForm,
    pub io_tasks: IoTasksForm,
    pub csi_delivery: CsiDeliveryForm,
    pub start_duration_seconds: String,
}

/// Ephemeral UI state that is not part of backend/device config.
#[derive(Debug, Clone)]
pub struct TransientUiState {
    pub active_tab: Tab,
    pub status_message: String,
    pub error_message: String,
    pub auto_scroll_stream: bool,
}

impl Default for TransientUiState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Dashboard,
            status_message: "Ready".to_owned(),
            error_message: String::new(),
            auto_scroll_stream: true,
        }
    }
}

/// Lightweight frame metadata shown in the Stream tab.
#[derive(Debug, Clone, Default)]
pub struct FrameSummary {
    pub timestamp: String,
    pub length: usize,
    pub preview_hex: String,
}

/// Runtime status produced by background IO work and `/api/control/status`.
#[derive(Debug, Clone, Default)]
pub struct RuntimeState {
    pub ws_connected: bool,
    pub serial_connected: Option<bool>,
    pub collection_running: Option<bool>,
    pub port_path: Option<String>,
    pub firmware_verified: Option<bool>,
    pub frames_received: u64,
    pub bytes_received: u64,
    pub recent_frames: Vec<FrameSummary>,
    pub events: Vec<String>,
    pub last_http_status: Option<u16>,
    pub latest_config: Option<DeviceConfig>,
    pub latest_info: Option<DeviceInfo>,
    /// Guards against an infinite reset/fetch loop when an empty fetch
    /// auto-issues `/api/config/reset` and the follow-up fetch is also empty.
    pub auto_resetting_cache: bool,
}

/// High-level user actions queued by the UI for orchestration.
#[derive(Debug, Clone)]
pub enum UserIntent {
    FetchConfig,
    FetchInfo,
    FetchStatus,
    ResetConfig,
    SetWifi(WiFiForm),
    SetTraffic(TrafficForm),
    SetCsi(CsiForm),
    SetCollectionMode(CollectionMode),
    SetLogMode(LogMode),
    SetOutputMode(OutputMode),
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
}

/// Wi-Fi section of `GET /api/config`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceWifiConfig {
    pub mode: Option<String>,
    pub channel: Option<u16>,
    pub sta_ssid: Option<String>,
}

/// Collection section of `GET /api/config`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCollectionConfig {
    pub mode: Option<String>,
    pub traffic_hz: Option<u64>,
    pub phy_rate: Option<String>,
    pub io_tx_enabled: Option<bool>,
    pub io_rx_enabled: Option<bool>,
}

/// CSI section of `GET /api/config`.
///
/// Mirrors firmware `show-config`: classic-chip booleans, HE-chip
/// `acquire_csi*` integers, plus read-only fields the device exposes
/// but does not accept via `POST /api/config/csi`.
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
    pub acquire_csi: Option<u32>,
    pub acquire_csi_legacy: Option<u32>,
    pub acquire_csi_ht20: Option<u32>,
    pub acquire_csi_ht40: Option<u32>,
    pub acquire_csi_su: Option<u32>,
    pub acquire_csi_mu: Option<u32>,
    pub acquire_csi_dcm: Option<u32>,
    pub acquire_csi_beamformed: Option<u32>,
    pub csi_he_stbc: Option<u32>,
    pub val_scale_cfg: Option<u32>,
}

/// Cached server-side device configuration model.
///
/// Mirrors `GET /api/config`. Sub-sections are `Option` so an explicit
/// `null` from the server (cache-not-yet-populated) deserializes as
/// `None` instead of erroring the whole payload out.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    #[serde(default)]
    pub wifi: Option<DeviceWifiConfig>,
    #[serde(default)]
    pub collection: Option<DeviceCollectionConfig>,
    #[serde(default)]
    pub csi_config: Option<DeviceCsiConfig>,
    pub log_mode: Option<String>,
    pub csi_delivery_mode: Option<String>,
    pub csi_logging_enabled: Option<bool>,
}

/// Firmware identity from `GET /api/info`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceInfo {
    pub banner_version: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub chip: Option<String>,
    pub protocol: Option<u32>,
    #[serde(default)]
    pub features: Vec<String>,
}

/// Runtime status payload from `GET /api/control/status`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlStatus {
    pub serial_connected: Option<bool>,
    pub collection_running: Option<bool>,
    pub port_path: Option<String>,
}

/// Full application state.
///
/// This is the single source of truth for all UI-visible data.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub persistent: PersistentState,
    pub transient: TransientUiState,
    pub runtime: RuntimeState,
    intent_queue: Vec<UserIntent>,
}

impl AppState {
    /// Construct default state with localhost webserver settings.
    pub fn with_defaults() -> Self {
        let mut state = Self::default();
        state.persistent.server_host = "127.0.0.1".to_owned();
        state.persistent.server_port = "3000".to_owned();
        state
    }

    /// Queue one user intent.
    pub fn push_intent(&mut self, intent: UserIntent) {
        self.intent_queue.push(intent);
    }

    /// Drain queued intents in FIFO order.
    pub fn drain_intents(&mut self) -> Vec<UserIntent> {
        std::mem::take(&mut self.intent_queue)
    }

    /// Append one event line to runtime history.
    pub fn push_event(&mut self, message: impl Into<String>) {
        self.runtime.events.push(message.into());
        if self.runtime.events.len() > 300 {
            let drain_to = self.runtime.events.len() - 300;
            self.runtime.events.drain(0..drain_to);
        }
    }

    /// Record one received frame and update stream counters/history.
    pub fn push_frame(&mut self, bytes: &[u8]) {
        self.runtime.frames_received = self.runtime.frames_received.saturating_add(1);
        self.runtime.bytes_received = self.runtime.bytes_received.saturating_add(bytes.len() as u64);

        let preview = bytes
            .iter()
            .take(24)
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        self.runtime.recent_frames.push(FrameSummary {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            length: bytes.len(),
            preview_hex: preview,
        });

        if self.runtime.recent_frames.len() > 300 {
            let drain_to = self.runtime.recent_frames.len() - 300;
            self.runtime.recent_frames.drain(0..drain_to);
        }
    }

    /// Build HTTP base URL from host/port fields.
    pub fn base_http_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.persistent.server_host.trim(),
            self.persistent.server_port.trim()
        )
    }

    /// Build WebSocket stream URL from host/port fields.
    pub fn base_ws_url(&self) -> String {
        format!(
            "ws://{}:{}/api/ws",
            self.persistent.server_host.trim(),
            self.persistent.server_port.trim()
        )
    }

    /// Apply server config payload into local persistent state fields.
    ///
    /// Returns the number of fields that were actually applied; callers
    /// use a zero return to detect an empty server cache.
    pub fn apply_device_config(&mut self, config: DeviceConfig) -> usize {
        let mut applied = 0;

        if let Some(wifi) = config.wifi.as_ref() {
            if let Some(mode) = wifi.mode.as_deref() {
                if let Some(parsed) = WiFiMode::from_api_value(mode) {
                    self.persistent.wifi.mode = parsed;
                    applied += 1;
                }
            }
            if let Some(channel) = wifi.channel {
                self.persistent.wifi.channel = channel.to_string();
                applied += 1;
            }
            if let Some(ssid) = &wifi.sta_ssid {
                self.persistent.wifi.sta_ssid = ssid.clone();
                applied += 1;
            }
        }

        if let Some(collection) = config.collection.as_ref() {
            if let Some(traffic_hz) = collection.traffic_hz {
                self.persistent.traffic.frequency_hz = traffic_hz.to_string();
                applied += 1;
            }
            if let Some(mode) = collection.mode.as_deref() {
                self.persistent.collection_mode = if mode == "listener" {
                    CollectionMode::Listener
                } else {
                    CollectionMode::Collector
                };
                applied += 1;
            }
            if let Some(rate) = &collection.phy_rate {
                self.persistent.phy_rate.rate = rate.clone();
                applied += 1;
            }
            if let Some(tx) = collection.io_tx_enabled {
                self.persistent.io_tasks.tx = tx;
                applied += 1;
            }
            if let Some(rx) = collection.io_rx_enabled {
                self.persistent.io_tasks.rx = rx;
                applied += 1;
            }
        }

        if let Some(csi_cfg) = config.csi_config.as_ref() {
            if let Some(v) = csi_cfg.lltf_enabled {
                self.persistent.csi.disable_lltf = !v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.htltf_enabled {
                self.persistent.csi.disable_htltf = !v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.stbc_htltf_enabled {
                self.persistent.csi.disable_stbc_htltf = !v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.ltf_merge_enabled {
                self.persistent.csi.disable_ltf_merge = !v;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi {
                self.persistent.csi.disable_csi = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_legacy {
                self.persistent.csi.disable_csi_legacy = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_ht20 {
                self.persistent.csi.disable_csi_ht20 = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_ht40 {
                self.persistent.csi.disable_csi_ht40 = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_su {
                self.persistent.csi.disable_csi_su = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_mu {
                self.persistent.csi.disable_csi_mu = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_dcm {
                self.persistent.csi.disable_csi_dcm = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.acquire_csi_beamformed {
                self.persistent.csi.disable_csi_beamformed = v == 0;
                applied += 1;
            }
            if let Some(v) = csi_cfg.csi_he_stbc {
                self.persistent.csi.csi_he_stbc = v.to_string();
                applied += 1;
            }
            if let Some(v) = csi_cfg.val_scale_cfg {
                self.persistent.csi.val_scale_cfg = v.to_string();
                applied += 1;
            }
        }

        if let Some(mode) = config.log_mode.as_deref() {
            self.persistent.log_mode = match mode {
                "text" => LogMode::Text,
                "serialized" => LogMode::Serialized,
                "esp-csi-tool" => LogMode::EspCsiTool,
                _ => LogMode::ArrayList,
            };
            applied += 1;
        }

        if let Some(mode) = config.csi_delivery_mode.as_deref() {
            self.persistent.csi_delivery.mode = match mode {
                "off" => CsiDeliveryMode::Off,
                "callback" => CsiDeliveryMode::Callback,
                _ => CsiDeliveryMode::Async,
            };
            applied += 1;
        }
        if let Some(logging) = config.csi_logging_enabled {
            self.persistent.csi_delivery.logging = logging;
            applied += 1;
        }

        self.runtime.latest_config = Some(config);
        applied
    }

    /// Apply a `/api/control/status` payload to runtime state.
    pub fn apply_control_status(&mut self, status: ControlStatus) {
        self.runtime.serial_connected = status.serial_connected;
        self.runtime.collection_running = status.collection_running;
        self.runtime.port_path = status.port_path;
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
                "mode": "collector", "traffic_hz": 100, "phy_rate": "mcs0-lgi",
                "io_tx_enabled": true, "io_rx_enabled": true
            },
            "csi_config": {
                "lltf_enabled": true, "htltf_enabled": true,
                "stbc_htltf_enabled": true, "ltf_merge_enabled": true,
                "csi_he_stbc": 2, "val_scale_cfg": 2,
                "acquire_csi": 1, "acquire_csi_legacy": 0
            },
            "log_mode": "array-list",
            "csi_delivery_mode": "async",
            "csi_logging_enabled": true
        }"#;
        let cfg: DeviceConfig = serde_json::from_str(json).expect("parse");
        let mut state = AppState::with_defaults();
        let applied = state.apply_device_config(cfg);
        assert!(applied > 0);
        assert_eq!(state.persistent.wifi.mode, WiFiMode::Sniffer);
        assert_eq!(state.persistent.wifi.channel, "6");
        assert_eq!(state.persistent.traffic.frequency_hz, "100");
        assert!(!state.persistent.csi.disable_csi);
        assert!(state.persistent.csi.disable_csi_legacy);
    }

    #[test]
    fn device_config_tolerates_null_sub_objects() {
        let json = r#"{ "wifi": null, "collection": null, "csi_config": null }"#;
        let cfg: DeviceConfig = serde_json::from_str(json).expect("parse null subobjects");
        let mut state = AppState::with_defaults();
        assert_eq!(state.apply_device_config(cfg), 0);
    }

    #[test]
    fn device_config_tolerates_missing_sub_objects() {
        let cfg: DeviceConfig = serde_json::from_str("{}").expect("parse empty");
        let mut state = AppState::with_defaults();
        assert_eq!(state.apply_device_config(cfg), 0);
    }
}
