use crate::core::messages::{ApiRequest, CoreCommand, CoreEvent, HttpMethod};
use crate::core::CoreHandle;
use crate::state::{AppState, DeviceConfig, Tab, UserIntent};
use crate::ui;
use eframe::egui;
use serde_json::json;

/// Top-level egui application.
///
/// This type orchestrates the intent-command-event flow:
///
/// - reads and drains user intents from [`crate::state::AppState`]
/// - submits commands to [`crate::core::CoreHandle`]
/// - applies resulting core events back into state
pub struct CsiClientApp {
    state: AppState,
    core: CoreHandle,
}

impl CsiClientApp {
    /// Create a new app instance with default state and a running core worker.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::with_defaults(),
            core: CoreHandle::new(),
        }
    }

    /// Drain queued user intents and translate them into core commands.
    ///
    /// This keeps network and runtime side effects out of the UI modules.
    fn process_intents(&mut self) {
        for intent in self.state.drain_intents() {
            match intent {
                UserIntent::FetchConfig => {
                    self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                        label: "fetch_config".to_owned(),
                        method: HttpMethod::Get,
                        base_url: self.state.base_http_url(),
                        path: "/api/config".to_owned(),
                        body: None,
                    }));
                }
                UserIntent::ResetConfig => {
                    self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                        label: "reset_config".to_owned(),
                        method: HttpMethod::Post,
                        base_url: self.state.base_http_url(),
                        path: "/api/config/reset".to_owned(),
                        body: None,
                    }));
                }
                UserIntent::SetWifi(wifi) => {
                    let channel = parse_optional_u16(&wifi.channel);
                    if wifi.channel.trim().is_empty() || channel.is_some() {
                        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                            label: "set_wifi".to_owned(),
                            method: HttpMethod::Post,
                            base_url: self.state.base_http_url(),
                            path: "/api/config/wifi".to_owned(),
                            body: Some(json!({
                                "mode": wifi.mode.as_api_value(),
                                "sta_ssid": empty_to_none(wifi.sta_ssid),
                                "sta_password": empty_to_none(wifi.sta_password),
                                "channel": channel,
                            })),
                        }));
                    } else {
                        self.state.transient.error_message =
                            "Wi-Fi channel must be a valid number".to_owned();
                    }
                }
                UserIntent::SetTraffic(traffic) => {
                    if let Some(frequency_hz) = parse_required_u16(&traffic.frequency_hz) {
                        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                            label: "set_traffic".to_owned(),
                            method: HttpMethod::Post,
                            base_url: self.state.base_http_url(),
                            path: "/api/config/traffic".to_owned(),
                            body: Some(json!({ "frequency_hz": frequency_hz })),
                        }));
                    } else {
                        self.state.transient.error_message =
                            "Traffic frequency must be a valid number".to_owned();
                    }
                }
                UserIntent::SetCsi(csi) => {
                    let csi_he_stbc = parse_required_u8(&csi.csi_he_stbc);
                    let val_scale_cfg = parse_required_u8(&csi.val_scale_cfg);

                    if let (Some(csi_he_stbc), Some(val_scale_cfg)) = (csi_he_stbc, val_scale_cfg) {
                        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                            label: "set_csi".to_owned(),
                            method: HttpMethod::Post,
                            base_url: self.state.base_http_url(),
                            path: "/api/config/csi".to_owned(),
                            body: Some(json!({
                                "disable_lltf": csi.disable_lltf,
                                "disable_htltf": csi.disable_htltf,
                                "disable_stbc_htltf": csi.disable_stbc_htltf,
                                "disable_ltf_merge": csi.disable_ltf_merge,
                                "disable_csi": csi.disable_csi,
                                "disable_csi_legacy": csi.disable_csi_legacy,
                                "disable_csi_ht20": csi.disable_csi_ht20,
                                "disable_csi_ht40": csi.disable_csi_ht40,
                                "disable_csi_su": csi.disable_csi_su,
                                "disable_csi_mu": csi.disable_csi_mu,
                                "disable_csi_dcm": csi.disable_csi_dcm,
                                "disable_csi_beamformed": csi.disable_csi_beamformed,
                                "csi_he_stbc": csi_he_stbc,
                                "val_scale_cfg": val_scale_cfg
                            })),
                        }));
                    } else {
                        self.state.transient.error_message =
                            "CSI u8 fields must be valid numbers in 0..255".to_owned();
                    }
                }
                UserIntent::SetCollectionMode(mode) => {
                    self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                        label: "set_collection_mode".to_owned(),
                        method: HttpMethod::Post,
                        base_url: self.state.base_http_url(),
                        path: "/api/config/collection-mode".to_owned(),
                        body: Some(json!({ "mode": mode.as_api_value() })),
                    }));
                }
                UserIntent::SetLogMode(mode) => {
                    self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                        label: "set_log_mode".to_owned(),
                        method: HttpMethod::Post,
                        base_url: self.state.base_http_url(),
                        path: "/api/config/log-mode".to_owned(),
                        body: Some(json!({ "mode": mode.as_api_value() })),
                    }));
                }
                UserIntent::SetOutputMode(mode) => {
                    self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                        label: "set_output_mode".to_owned(),
                        method: HttpMethod::Post,
                        base_url: self.state.base_http_url(),
                        path: "/api/config/output-mode".to_owned(),
                        body: Some(json!({ "mode": mode.as_api_value() })),
                    }));
                }
                UserIntent::StartCollection { duration_seconds } => {
                    let duration = parse_optional_u64(&duration_seconds);
                    if duration_seconds.trim().is_empty() || duration.is_some() {
                        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                            label: "start_collection".to_owned(),
                            method: HttpMethod::Post,
                            base_url: self.state.base_http_url(),
                            path: "/api/control/start".to_owned(),
                            body: duration.map(|d| json!({ "duration": d })),
                        }));
                    } else {
                        self.state.transient.error_message =
                            "Duration must be a valid number of seconds".to_owned();
                    }
                }
                UserIntent::ResetDevice => {
                    self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
                        label: "reset_device".to_owned(),
                        method: HttpMethod::Post,
                        base_url: self.state.base_http_url(),
                        path: "/api/control/reset".to_owned(),
                        body: None,
                    }));
                }
                UserIntent::ConnectWebSocket => {
                    self.core.submit(CoreCommand::ConnectWebSocket {
                        url: self.state.base_ws_url(),
                    });
                }
                UserIntent::DisconnectWebSocket => {
                    self.core.submit(CoreCommand::DisconnectWebSocket);
                }
                UserIntent::ClearFrames => {
                    self.state.runtime.recent_frames.clear();
                    self.state.runtime.frames_received = 0;
                    self.state.runtime.bytes_received = 0;
                }
            }
        }
    }

    /// Poll and apply core worker events without blocking the frame loop.
    fn process_core_events(&mut self) {
        while let Some(event) = self.core.try_recv() {
            match event {
                CoreEvent::ApiResponse(response) => {
                    self.state.runtime.last_http_status = Some(response.status);

                    if response.success {
                        self.state.transient.status_message = format!(
                            "{} (HTTP {}): {}",
                            response.label, response.status, response.message
                        );
                        self.state.transient.error_message.clear();
                    } else {
                        self.state.transient.error_message = format!(
                            "{} failed (HTTP {}): {}",
                            response.label, response.status, response.message
                        );
                    }

                    self.state.push_event(format!(
                        "{} -> HTTP {}: {}",
                        response.label, response.status, response.message
                    ));

                    if response.label == "fetch_config" {
                        if let Some(data) = response.data {
                            if let Some(config) = parse_device_config(data) {
                                self.state.apply_device_config(config);
                            }
                        }
                    }
                }
                CoreEvent::WebSocketConnected => {
                    self.state.runtime.ws_connected = true;
                    self.state.transient.status_message = "WebSocket connected".to_owned();
                    self.state.transient.error_message.clear();
                    self.state.push_event("WebSocket connected");
                }
                CoreEvent::WebSocketDisconnected { reason } => {
                    self.state.runtime.ws_connected = false;
                    self.state.push_event(format!("WebSocket disconnected: {reason}"));
                }
                CoreEvent::WebSocketFrame(bytes) => {
                    self.state.push_frame(&bytes);
                }
                CoreEvent::Log(line) => {
                    self.state.push_event(line);
                }
            }
        }
    }

    /// Render the shared top panel (host/port fields, tabs, status and errors).
    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Host");
                ui.text_edit_singleline(&mut self.state.persistent.server_host);
                ui.label("Port");
                ui.text_edit_singleline(&mut self.state.persistent.server_port);
                if ui.button("Fetch Config").clicked() {
                    self.state.push_intent(UserIntent::FetchConfig);
                }
            });

            ui.horizontal(|ui| {
                tab_button(ui, &mut self.state, Tab::Dashboard, "Dashboard");
                tab_button(ui, &mut self.state, Tab::Config, "Config");
                tab_button(ui, &mut self.state, Tab::Control, "Control");
                tab_button(ui, &mut self.state, Tab::Stream, "Stream");
            });

            if !self.state.transient.status_message.is_empty() {
                ui.label(format!("Status: {}", self.state.transient.status_message));
            }

            if !self.state.transient.error_message.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 80, 80),
                    format!("Error: {}", self.state.transient.error_message),
                );
            }
        });
    }
}

impl eframe::App for CsiClientApp {
    /// Main egui frame update callback.
    ///
    /// The update order is:
    /// 1. apply incoming core events
    /// 2. process queued user intents
    /// 3. render UI panels
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_core_events();
        self.process_intents();

        self.render_top_bar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| match self.state.transient.active_tab {
            Tab::Dashboard => ui::dashboard::render(ui, &mut self.state),
            Tab::Config => ui::config::render(ui, &mut self.state),
            Tab::Control => ui::control::render(ui, &mut self.state),
            Tab::Stream => ui::stream::render(ui, &mut self.state),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Parse a `DeviceConfig` from a direct payload or common API envelope.
fn parse_device_config(data: serde_json::Value) -> Option<DeviceConfig> {
    if let Ok(config) = serde_json::from_value::<DeviceConfig>(data.clone()) {
        return Some(config);
    }

    if let Some(inner) = data.get("data") {
        return serde_json::from_value::<DeviceConfig>(inner.clone()).ok();
    }

    None
}

/// Parse an optional `u16` where empty input means `None`.
fn parse_optional_u16(input: &str) -> Option<u16> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u16>().ok()
}

/// Parse a required `u16`.
fn parse_required_u16(input: &str) -> Option<u16> {
    input.trim().parse::<u16>().ok()
}

/// Parse a required `u8`.
fn parse_required_u8(input: &str) -> Option<u8> {
    input.trim().parse::<u8>().ok()
}

/// Parse an optional `u64` where empty input means `None`.
fn parse_optional_u64(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// Convert user text to optional string while preserving significant whitespace.
///
/// Returns `None` only when the input is entirely whitespace.
fn empty_to_none(input: String) -> Option<String> {
    if input.trim().is_empty() {
        None
    } else {
        Some(input)
    }
}

/// Render one tab selector button and switch active tab on click.
fn tab_button(ui: &mut egui::Ui, state: &mut AppState, tab: Tab, label: &str) {
    let selected = state.transient.active_tab == tab;
    if ui.selectable_label(selected, label).clicked() {
        state.transient.active_tab = tab;
    }
}
