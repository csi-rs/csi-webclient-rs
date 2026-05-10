use crate::core::CoreHandle;
use crate::core::messages::{ApiRequest, CoreCommand, CoreEvent, HttpMethod};
use crate::state::{
    AppState, ControlStatus, DeviceConfig, DeviceInfo, Tab, UserIntent, WiFiForm,
};
use crate::ui;
use eframe::egui;
use serde_json::{Value, json};

const STA_FIELD_MAX_BYTES: usize = 32;

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
    fn process_intents(&mut self) {
        for intent in self.state.drain_intents() {
            match intent {
                UserIntent::FetchConfig => self.submit_get("fetch_config", "/api/config"),
                UserIntent::FetchInfo => self.submit_get("fetch_info", "/api/info"),
                UserIntent::FetchStatus => {
                    self.submit_get("fetch_status", "/api/control/status");
                }
                UserIntent::ResetConfig => self.submit_post("reset_config", "/api/config/reset", None),
                UserIntent::SetWifi(wifi) => self.submit_set_wifi(wifi),
                UserIntent::SetTraffic(traffic) => {
                    if let Some(frequency_hz) = parse_required_u64(&traffic.frequency_hz) {
                        self.submit_post(
                            "set_traffic",
                            "/api/config/traffic",
                            Some(json!({ "frequency_hz": frequency_hz })),
                        );
                    } else {
                        self.set_error("Traffic frequency must be a non-negative integer");
                    }
                }
                UserIntent::SetCsi(csi) => {
                    let csi_he_stbc = parse_required_u32(&csi.csi_he_stbc);
                    let val_scale_cfg = parse_required_u32(&csi.val_scale_cfg);

                    if let (Some(csi_he_stbc), Some(val_scale_cfg)) = (csi_he_stbc, val_scale_cfg) {
                        self.submit_post(
                            "set_csi",
                            "/api/config/csi",
                            Some(json!({
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
                                "val_scale_cfg": val_scale_cfg,
                            })),
                        );
                    } else {
                        self.set_error("csi_he_stbc and val_scale_cfg must be valid u32 numbers");
                    }
                }
                UserIntent::SetCollectionMode(mode) => {
                    self.submit_post(
                        "set_collection_mode",
                        "/api/config/collection-mode",
                        Some(json!({ "mode": mode.as_api_value() })),
                    );
                }
                UserIntent::SetLogMode(mode) => {
                    self.submit_post(
                        "set_log_mode",
                        "/api/config/log-mode",
                        Some(json!({ "mode": mode.as_api_value() })),
                    );
                }
                UserIntent::SetOutputMode(mode) => {
                    self.submit_post(
                        "set_output_mode",
                        "/api/config/output-mode",
                        Some(json!({ "mode": mode.as_api_value() })),
                    );
                }
                UserIntent::SetPhyRate(form) => {
                    let rate = form.rate.trim().to_owned();
                    if rate.is_empty() {
                        self.set_error("PHY rate must not be empty");
                    } else {
                        self.submit_post(
                            "set_rate",
                            "/api/config/rate",
                            Some(json!({ "rate": rate })),
                        );
                    }
                }
                UserIntent::SetIoTasks(form) => {
                    self.submit_post(
                        "set_io_tasks",
                        "/api/config/io-tasks",
                        Some(json!({ "tx": form.tx, "rx": form.rx })),
                    );
                }
                UserIntent::SetCsiDelivery(form) => {
                    self.submit_post(
                        "set_csi_delivery",
                        "/api/config/csi-delivery",
                        Some(json!({
                            "mode": form.mode.as_api_value(),
                            "logging": form.logging,
                        })),
                    );
                }
                UserIntent::StartCollection { duration_seconds } => {
                    let duration = parse_optional_u64(&duration_seconds);
                    if duration_seconds.trim().is_empty() || duration.is_some() {
                        self.submit_post(
                            "start_collection",
                            "/api/control/start",
                            duration.map(|d| json!({ "duration": d })),
                        );
                    } else {
                        self.set_error("Duration must be a valid number of seconds");
                    }
                }
                UserIntent::StopCollection => {
                    self.submit_post("stop_collection", "/api/control/stop", None);
                }
                UserIntent::ShowStats => {
                    self.submit_post("show_stats", "/api/control/stats", None);
                }
                UserIntent::ResetDevice => {
                    self.submit_post("reset_device", "/api/control/reset", None);
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

    fn submit_get(&self, label: &str, path: &str) {
        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
            label: label.to_owned(),
            method: HttpMethod::Get,
            base_url: self.state.base_http_url(),
            path: path.to_owned(),
            body: None,
        }));
    }

    fn submit_post(&self, label: &str, path: &str, body: Option<Value>) {
        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
            label: label.to_owned(),
            method: HttpMethod::Post,
            base_url: self.state.base_http_url(),
            path: path.to_owned(),
            body,
        }));
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.state.transient.error_message = message.into();
    }

    fn submit_set_wifi(&mut self, wifi: WiFiForm) {
        if let Err(message) = validate_sta_field("STA SSID", &wifi.sta_ssid) {
            self.set_error(message);
            return;
        }
        if let Err(message) = validate_sta_field("STA password", &wifi.sta_password) {
            self.set_error(message);
            return;
        }

        let channel = parse_optional_u16(&wifi.channel);
        if !wifi.channel.trim().is_empty() && channel.is_none() {
            self.set_error("Wi-Fi channel must be a valid number");
            return;
        }

        self.submit_post(
            "set_wifi",
            "/api/config/wifi",
            Some(json!({
                "mode": wifi.mode.as_api_value(),
                "sta_ssid": empty_to_none(&wifi.sta_ssid),
                "sta_password": empty_to_none(&wifi.sta_password),
                "channel": channel,
            })),
        );
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
                        self.state.transient.error_message = format_error(
                            &response.label,
                            response.status,
                            &response.message,
                        );
                    }

                    self.state.push_event(format!(
                        "{} -> HTTP {}: {}",
                        response.label, response.status, response.message
                    ));

                    if response.success {
                        match response.label.as_str() {
                            "fetch_config" => {
                                if let Some(config) =
                                    response.data.and_then(parse_envelope::<DeviceConfig>)
                                {
                                    let applied = self.state.apply_device_config(config);
                                    if applied == 0 && !self.state.runtime.auto_resetting_cache {
                                        self.state.runtime.auto_resetting_cache = true;
                                        self.state.push_intent(UserIntent::ResetConfig);
                                    } else {
                                        self.state.runtime.auto_resetting_cache = false;
                                    }
                                } else {
                                    self.state.runtime.auto_resetting_cache = false;
                                }
                            }
                            "fetch_info" => {
                                if let Some(info) =
                                    response.data.and_then(parse_envelope::<DeviceInfo>)
                                {
                                    self.state.runtime.firmware_verified = Some(true);
                                    self.state.runtime.latest_info = Some(info);
                                }
                            }
                            "fetch_status" => {
                                if let Some(status) =
                                    response.data.and_then(parse_envelope::<ControlStatus>)
                                {
                                    self.state.apply_control_status(status);
                                }
                            }
                            "start_collection" => {
                                self.state.runtime.collection_running = Some(true);
                            }
                            "stop_collection" => {
                                self.state.runtime.collection_running = Some(false);
                            }
                            "reset_device" => {
                                self.state.runtime.collection_running = Some(false);
                                self.state.runtime.firmware_verified = None;
                                self.state.runtime.latest_info = None;
                            }
                            // Any successful config-mutating POST repopulates a slot in the
                            // server cache, so re-pull `/api/config` to keep the form in sync.
                            "reset_config"
                            | "set_wifi"
                            | "set_traffic"
                            | "set_csi"
                            | "set_collection_mode"
                            | "set_log_mode"
                            | "set_rate"
                            | "set_io_tasks"
                            | "set_csi_delivery" => {
                                self.state.push_intent(UserIntent::FetchConfig);
                            }
                            _ => {}
                        }
                    } else if response.label == "fetch_info" && response.status != 0 {
                        self.state.runtime.firmware_verified = Some(false);
                    } else if response.label == "reset_config" {
                        self.state.runtime.auto_resetting_cache = false;
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
            ui.horizontal_wrapped(|ui| {
                ui.label("Host");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.persistent.server_host)
                        .desired_width(140.0),
                );
                ui.label("Port");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.persistent.server_port)
                        .desired_width(60.0),
                );
                if ui.button("Fetch Info").clicked() {
                    self.state.push_intent(UserIntent::FetchInfo);
                }
                if ui.button("Fetch Config").clicked() {
                    self.state.push_intent(UserIntent::FetchConfig);
                }
                if ui.button("Fetch Status").clicked() {
                    self.state.push_intent(UserIntent::FetchStatus);
                }
            });

            ui.horizontal_wrapped(|ui| {
                tab_button(ui, &mut self.state, Tab::Dashboard, "Dashboard");
                tab_button(ui, &mut self.state, Tab::Config, "Config");
                tab_button(ui, &mut self.state, Tab::Control, "Control");
                tab_button(ui, &mut self.state, Tab::Stream, "Stream");
            });

            if !self.state.transient.status_message.is_empty() {
                ui.add(
                    egui::Label::new(format!("Status: {}", self.state.transient.status_message))
                        .wrap(),
                );
            }

            if !self.state.transient.error_message.is_empty() {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!(
                            "Error: {}",
                            self.state.transient.error_message
                        ))
                        .color(egui::Color32::from_rgb(220, 80, 80)),
                    )
                    .wrap(),
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

/// Parse a typed payload from a direct value or the standard `data` envelope.
fn parse_envelope<T: serde::de::DeserializeOwned>(data: serde_json::Value) -> Option<T> {
    if let Ok(value) = serde_json::from_value::<T>(data.clone()) {
        return Some(value);
    }
    if let Some(inner) = data.get("data") {
        return serde_json::from_value::<T>(inner.clone()).ok();
    }
    None
}

/// Reject SSID/password values the firmware tokenizer cannot accept.
///
/// Mirrors the server-side rules from §1.4 of the webserver spec so users
/// see the failure inline rather than as a 400 round-trip.
fn validate_sta_field(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    if value.len() > STA_FIELD_MAX_BYTES {
        return Err(format!("{label} exceeds 32-byte firmware limit"));
    }
    if value.contains('\r') || value.contains('\n') {
        return Err(format!("{label} must not contain newlines"));
    }
    if value.contains('\'') && value.contains('"') {
        return Err(format!(
            "{label} cannot contain both ' and \" — firmware tokenizer cannot disambiguate"
        ));
    }
    Ok(())
}

/// Map known status codes onto operator-friendly hints.
fn format_error(label: &str, status: u16, message: &str) -> String {
    let hint = match status {
        412 => Some("firmware not verified — try Fetch Info or Reset Device"),
        503 => Some("ESP32 not connected, or operation not valid for current state"),
        502 => Some("device responded but the info block was malformed"),
        504 => Some("info block timed out — firmware may not be esp-csi-cli-rs"),
        403 => Some("output mode is dump — switch to stream/both before opening WebSocket"),
        _ => None,
    };
    match hint {
        Some(h) => format!("{label} failed (HTTP {status}): {message} — {h}"),
        None => format!("{label} failed (HTTP {status}): {message}"),
    }
}

/// Parse an optional `u16` where empty input means `None`.
fn parse_optional_u16(input: &str) -> Option<u16> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u16>().ok()
}

/// Parse a required `u64`.
fn parse_required_u64(input: &str) -> Option<u64> {
    input.trim().parse::<u64>().ok()
}

/// Parse a required `u32`.
fn parse_required_u32(input: &str) -> Option<u32> {
    input.trim().parse::<u32>().ok()
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
fn empty_to_none(input: &str) -> Option<String> {
    if input.trim().is_empty() {
        None
    } else {
        Some(input.to_owned())
    }
}

/// Render one tab selector button and switch active tab on click.
fn tab_button(ui: &mut egui::Ui, state: &mut AppState, tab: Tab, label: &str) {
    let selected = state.transient.active_tab == tab;
    if ui.selectable_label(selected, label).clicked() {
        state.transient.active_tab = tab;
    }
}
