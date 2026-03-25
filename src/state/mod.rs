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

/// Wi-Fi operating modes supported by the backend API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WiFiMode {
    Sta,
    Monitor,
    Sniffer,
}

impl WiFiMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Sta => "sta",
            Self::Monitor => "monitor",
            Self::Sniffer => "sniffer",
        }
    }
}

impl Default for WiFiMode {
    fn default() -> Self {
        Self::Sta
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
}

impl LogMode {
    /// Convert enum variant to backend API value.
    pub fn as_api_value(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::ArrayList => "array-list",
            Self::Serialized => "serialized",
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
            csi_he_stbc: "0".to_owned(),
            val_scale_cfg: "0".to_owned(),
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

/// Runtime status produced by background IO work.
#[derive(Debug, Clone, Default)]
pub struct RuntimeState {
    pub ws_connected: bool,
    pub frames_received: u64,
    pub bytes_received: u64,
    pub recent_frames: Vec<FrameSummary>,
    pub events: Vec<String>,
    pub last_http_status: Option<u16>,
    pub latest_config: Option<DeviceConfig>,
}

/// High-level user actions queued by the UI for orchestration.
#[derive(Debug, Clone)]
pub enum UserIntent {
    FetchConfig,
    ResetConfig,
    SetWifi(WiFiForm),
    SetTraffic(TrafficForm),
    SetCsi(CsiForm),
    SetCollectionMode(CollectionMode),
    SetLogMode(LogMode),
    SetOutputMode(OutputMode),
    StartCollection { duration_seconds: String },
    ResetDevice,
    ConnectWebSocket,
    DisconnectWebSocket,
    ClearFrames,
}

/// Cached server-side device configuration model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    pub wifi_mode: Option<String>,
    pub channel: Option<u16>,
    pub sta_ssid: Option<String>,
    pub traffic_hz: Option<u16>,
    pub collection_mode: Option<String>,
    pub log_mode: Option<String>,
    pub log_format: Option<String>,
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
    pub fn apply_device_config(&mut self, config: DeviceConfig) {
        if let Some(mode) = config.wifi_mode.as_deref() {
            self.persistent.wifi.mode = match mode {
                "monitor" => WiFiMode::Monitor,
                "sniffer" => WiFiMode::Sniffer,
                _ => WiFiMode::Sta,
            };
        }

        if let Some(channel) = config.channel {
            self.persistent.wifi.channel = channel.to_string();
        }

        if let Some(ssid) = &config.sta_ssid {
            self.persistent.wifi.sta_ssid = ssid.clone();
        }

        if let Some(traffic_hz) = config.traffic_hz {
            self.persistent.traffic.frequency_hz = traffic_hz.to_string();
        }

        if let Some(mode) = config.collection_mode.as_deref() {
            self.persistent.collection_mode = if mode == "listener" {
                CollectionMode::Listener
            } else {
                CollectionMode::Collector
            };
        }

        if let Some(mode) = config.log_mode.as_deref().or(config.log_format.as_deref()) {
            self.persistent.log_mode = match mode {
                "text" => LogMode::Text,
                // Backward compatibility for older backend values.
                "cobs" | "serialized" => LogMode::Serialized,
                _ => LogMode::ArrayList,
            };
        }

        self.runtime.latest_config = Some(config);
    }
}
