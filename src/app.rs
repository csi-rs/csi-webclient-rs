use crate::core::CoreHandle;
use crate::core::messages::{ApiRequest, ApiResponseEvent, CoreCommand, CoreEvent, HttpMethod};
use crate::export::Recorder;
use crate::state::{
    AppState, ControlStatus, DeviceAction, DeviceConfig, DeviceInfo, DeviceListEntry,
    PairingPreset, ServerStatus, Tab, TrafficForm, UserIntent, WiFiForm, WiFiMode, WifiProtocol,
};
use crate::ui;
use eframe::egui;
use serde_json::{Value, json};
use std::collections::HashMap;

const STA_FIELD_MAX_BYTES: usize = 32;
/// How often to poll `GET /api/devices` for hotplug discovery.
const DEVICE_POLL_INTERVAL_SECS: f64 = 2.0;

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
    /// Wall-clock (egui time) of the last device-discovery poll.
    last_devices_poll: Option<f64>,
    /// Active Parquet recordings, keyed by device id. Held here (not in
    /// [`AppState`], which is `Clone`) because the file writer is not cloneable.
    recorders: HashMap<String, Recorder>,
    /// Sequential preset steps (device id, action); drained on each 2xx response.
    preset_queue: Vec<(String, DeviceAction)>,
}

impl CsiClientApp {
    /// Create a new app instance with default state and a running core worker.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::with_defaults(),
            core: CoreHandle::new(),
            last_devices_poll: None,
            recorders: HashMap::new(),
            preset_queue: Vec::new(),
        }
    }

    /// Drain queued user intents and translate them into core commands.
    fn process_intents(&mut self) {
        for intent in self.state.drain_intents() {
            match intent {
                UserIntent::FetchDevices => {
                    self.submit_get("fetch_devices", None, "/api/devices".to_owned());
                }
                UserIntent::ToggleDeviceSelection(id) => {
                    self.state.toggle_selection(id);
                }
                UserIntent::SelectAllDevices => {
                    self.state.selected_device_ids = self.device_ids();
                }
                UserIntent::ClearDeviceSelection => {
                    self.state.selected_device_ids.clear();
                }
                UserIntent::StartAllCollections { duration_seconds } => {
                    for id in self.device_ids() {
                        self.process_device_action(
                            id,
                            DeviceAction::StartCollection {
                                duration_seconds: duration_seconds.clone(),
                            },
                        );
                    }
                }
                UserIntent::StopAllCollections => {
                    for id in self.device_ids() {
                        self.process_device_action(id, DeviceAction::StopCollection);
                    }
                }
                UserIntent::StartSelectedCollections { duration_seconds } => {
                    for id in self.state.selected_device_ids.clone() {
                        self.process_device_action(
                            id,
                            DeviceAction::StartCollection {
                                duration_seconds: duration_seconds.clone(),
                            },
                        );
                    }
                }
                UserIntent::StopSelectedCollections => {
                    for id in self.state.selected_device_ids.clone() {
                        self.process_device_action(id, DeviceAction::StopCollection);
                    }
                }
                UserIntent::StartSelectedRecording => {
                    for id in self.state.selected_device_ids.clone() {
                        self.start_recording(&id);
                    }
                }
                UserIntent::StopSelectedRecording => {
                    for id in self.state.selected_device_ids.clone() {
                        self.stop_recording(&id);
                    }
                }
                UserIntent::Device { id, action } => self.process_device_action(id, action),
            }
        }
    }

    /// All currently known device ids (snapshot).
    fn device_ids(&self) -> Vec<String> {
        self.state.devices.iter().map(|d| d.id.clone()).collect()
    }

    /// Translate one per-device action into an HTTP request or WebSocket command.
    fn process_device_action(&mut self, id: String, action: DeviceAction) {
        match action {
            DeviceAction::FetchConfig => self.submit_device_get(&id, "fetch_config", "config"),
            DeviceAction::FetchInfo => self.submit_device_get(&id, "fetch_info", "info"),
            DeviceAction::FetchStatus => {
                self.submit_device_get(&id, "fetch_status", "control/status")
            }
            DeviceAction::ResetConfig => {
                self.submit_device_post(&id, "reset_config", "config/reset", None)
            }
            DeviceAction::SetWifi(wifi) => self.submit_set_wifi(&id, wifi),
            DeviceAction::SetTraffic(traffic) => {
                if let Some(frequency_hz) = parse_required_u64(&traffic.frequency_hz) {
                    // Send `unsolicited` only for the WiFi infra modes AND
                    // firmware ≥ 0.7.1: the flag is meaningless elsewhere,
                    // and older firmware's CLI rejects the whole set-traffic
                    // command on an unknown flag — silently discarding the
                    // frequency too. The stored form is kept in sync by
                    // SetWifi/SetTraffic dispatch, so the mode is current
                    // even mid-pairing-preset.
                    let send_unsolicited = self
                        .state
                        .devices
                        .iter()
                        .find(|d| d.id == id)
                        .is_some_and(|d| {
                            matches!(d.forms.wifi.mode, WiFiMode::WifiAp | WiFiMode::Station)
                                && d.supports_unsolicited()
                        });
                    let body = if send_unsolicited {
                        json!({ "frequency_hz": frequency_hz, "unsolicited": traffic.unsolicited })
                    } else {
                        json!({ "frequency_hz": frequency_hz })
                    };
                    if let Some(device) = self.state.device_mut_by_id(&id) {
                        device.forms.traffic = traffic.clone();
                    }
                    self.submit_device_post(&id, "set_traffic", "config/traffic", Some(body));
                } else {
                    self.set_error("Traffic frequency must be a non-negative integer");
                }
            }
            DeviceAction::SetCsi(csi) => {
                let csi_he_stbc = parse_required_u32(&csi.csi_he_stbc);
                let val_scale_cfg = parse_required_u32(&csi.val_scale_cfg);

                if let (Some(csi_he_stbc), Some(val_scale_cfg)) = (csi_he_stbc, val_scale_cfg) {
                    self.submit_device_post(
                        &id,
                        "set_csi",
                        "config/csi",
                        Some(json!({
                            "lltf": csi.lltf,
                            "htltf": csi.htltf,
                            "stbc_htltf": csi.stbc_htltf,
                            "ltf_merge": csi.ltf_merge,
                            "csi": csi.csi,
                            "csi_legacy": csi.csi_legacy,
                            "csi_ht20": csi.csi_ht20,
                            "csi_ht40": csi.csi_ht40,
                            "csi_su": csi.csi_su,
                            "csi_mu": csi.csi_mu,
                            "csi_dcm": csi.csi_dcm,
                            "csi_beamformed": csi.csi_beamformed,
                            "dump_ack": csi.dump_ack,
                            "csi_force_lltf": csi.csi_force_lltf,
                            "csi_vht": csi.csi_vht,
                            "csi_he_stbc": csi_he_stbc,
                            "val_scale_cfg": val_scale_cfg,
                        })),
                    );
                } else {
                    self.set_error("csi_he_stbc and val_scale_cfg must be valid u32 numbers");
                }
            }
            DeviceAction::SetCsiPreset(preset) => {
                self.submit_device_post(
                    &id,
                    "set_csi_preset",
                    "config/csi",
                    Some(json!({ "preset": preset })),
                );
            }
            DeviceAction::SetCollectionMode(mode) => {
                self.submit_device_post(
                    &id,
                    "set_collection_mode",
                    "config/collection-mode",
                    Some(json!({ "mode": mode.as_api_value() })),
                );
            }
            DeviceAction::SetOutputMode(mode) => {
                self.submit_device_post(
                    &id,
                    "set_output_mode",
                    "config/output-mode",
                    Some(json!({ "mode": mode.as_api_value() })),
                );
            }
            DeviceAction::SetProtocol(protocol) => {
                self.submit_device_post(
                    &id,
                    "set_protocol",
                    "config/protocol",
                    Some(json!({ "protocol": protocol.as_api_value() })),
                );
            }
            DeviceAction::SetPhyRate(form) => {
                let rate = form.rate.trim().to_owned();
                if rate.is_empty() {
                    self.set_error("PHY rate must not be empty");
                } else {
                    self.submit_device_post(
                        &id,
                        "set_rate",
                        "config/rate",
                        Some(json!({ "rate": rate })),
                    );
                }
            }
            DeviceAction::SetIoTasks(form) => {
                self.submit_device_post(
                    &id,
                    "set_io_tasks",
                    "config/io-tasks",
                    Some(json!({ "tx": form.tx, "rx": form.rx })),
                );
            }
            DeviceAction::SetCsiDelivery(form) => {
                self.submit_device_post(
                    &id,
                    "set_csi_delivery",
                    "config/csi-delivery",
                    Some(json!({
                        "mode": form.mode.as_api_value(),
                        "logging": form.logging,
                    })),
                );
            }
            DeviceAction::StartCollection { duration_seconds } => {
                let duration = parse_optional_u64(&duration_seconds);
                if duration_seconds.trim().is_empty() || duration.is_some() {
                    self.submit_device_post(
                        &id,
                        "start_collection",
                        "control/start",
                        duration.map(|d| json!({ "duration": d })),
                    );
                } else {
                    self.set_error("Duration must be a valid number of seconds");
                }
            }
            DeviceAction::StopCollection => {
                self.submit_device_post(&id, "stop_collection", "control/stop", None);
            }
            DeviceAction::ShowStats => {
                self.submit_device_post(&id, "show_stats", "control/stats", None);
            }
            DeviceAction::ResetDevice => {
                self.submit_device_post(&id, "reset_device", "control/reset", None);
            }
            DeviceAction::ConnectWebSocket => {
                self.core.submit(CoreCommand::ConnectWebSocket {
                    url: self.state.device_ws_url(&id),
                    device_id: id,
                });
            }
            DeviceAction::DisconnectWebSocket => {
                self.core
                    .submit(CoreCommand::DisconnectWebSocket { device_id: id });
            }
            DeviceAction::ClearFrames => {
                if let Some(device) = self.state.device_mut_by_id(&id) {
                    device.clear_frames();
                }
            }
            DeviceAction::StartRecording => self.start_recording(&id),
            DeviceAction::StopRecording => self.stop_recording(&id),
            DeviceAction::ApplyPairingPreset {
                preset,
                device_ids,
                channel,
            } => self.apply_pairing_preset(preset, device_ids, channel),
        }
    }

    /// Open a Parquet recording for `id`'s incoming CSI stream.
    ///
    /// Requires the device chip to be known (decode is chip-specific). Frames
    /// are only captured while the WebSocket is connected, so the UI nudges the
    /// user to connect first.
    fn start_recording(&mut self, id: &str) {
        if self.recorders.contains_key(id) {
            return;
        }
        let Some(device) = self.state.device_mut_by_id(id) else {
            return;
        };
        let Some(chip) = device.latest_info.as_ref().and_then(|i| i.chip.clone()) else {
            self.set_error("Device chip unknown — Fetch Info before recording");
            return;
        };

        let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let dir = self.state.export_dir.trim();
        let dir = if dir.is_empty() { "." } else { dir };
        let path = format!("{dir}/csi_export_{id}_{stamp}.parquet");

        match Recorder::start(&path, &chip) {
            Ok(recorder) => {
                let path = recorder.path().to_owned();
                self.recorders.insert(id.to_owned(), recorder);
                if let Some(device) = self.state.device_mut_by_id(id) {
                    device.recording = true;
                    device.record_path = Some(path.clone());
                    device.recorded_frames = 0;
                    device.record_decode_errors = 0;
                }
                self.state.transient.error_message.clear();
                self.state.transient.status_message = format!("Recording {id} → {path}");
                self.state.push_event(format!("[{id}] recording to {path}"));
            }
            Err(message) => self.set_error(format!("Could not start recording: {message}")),
        }
    }

    /// Stop and finalize `id`'s Parquet recording, if any.
    fn finalize_recording(&mut self, id: &str) {
        if let Some(recorder) = self.recorders.remove(id) {
            let frames = recorder.frames_written;
            let path = recorder.path().to_owned();
            if let Err(e) = recorder.finish() {
                self.set_error(format!("Failed to finalize {path}: {e}"));
            } else {
                self.state.transient.status_message =
                    format!("Saved {frames} frame(s) → {path}");
                self.state.push_event(format!("[{id}] recording saved: {path}"));
            }
        }
        if let Some(device) = self.state.device_mut_by_id(id) {
            device.recording = false;
        }
    }

    /// User-initiated stop of `id`'s recording.
    fn stop_recording(&mut self, id: &str) {
        self.finalize_recording(id);
    }

    fn submit_get(&self, label: &str, device_id: Option<String>, path: String) {
        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
            label: label.to_owned(),
            device_id,
            method: HttpMethod::Get,
            base_url: self.state.base_http_url(),
            path,
            body: None,
        }));
    }

    fn submit_post(&self, label: &str, device_id: Option<String>, path: String, body: Option<Value>) {
        self.core.submit(CoreCommand::ExecuteApi(ApiRequest {
            label: label.to_owned(),
            device_id,
            method: HttpMethod::Post,
            base_url: self.state.base_http_url(),
            path,
            body,
        }));
    }

    /// Submit a GET to `/api/devices/{id}/{suffix}`.
    fn submit_device_get(&self, id: &str, label: &str, suffix: &str) {
        self.submit_get(label, Some(id.to_owned()), format!("/api/devices/{id}/{suffix}"));
    }

    /// Submit a POST to `/api/devices/{id}/{suffix}`.
    fn submit_device_post(&self, id: &str, label: &str, suffix: &str, body: Option<Value>) {
        self.submit_post(
            label,
            Some(id.to_owned()),
            format!("/api/devices/{id}/{suffix}"),
            body,
        );
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.state.transient.error_message = message.into();
    }

    fn submit_set_wifi(&mut self, id: &str, wifi: WiFiForm) {
        if wifi.mode.requires_v07() {
            if let Some(device) = self.state.device_mut_by_id(id) {
                if !device.supports_v07_modes() {
                    self.set_error(format!(
                        "Mode '{}' requires esp-csi-cli-rs ≥ 0.7.0 on device {id}",
                        wifi.mode.as_api_value()
                    ));
                    return;
                }
            }
        }

        if matches!(wifi.mode, WiFiMode::Station) {
            if let Err(message) = validate_sta_field("STA SSID", &wifi.sta_ssid) {
                self.set_error(message);
                return;
            }
            if let Err(message) = validate_sta_field("STA password", &wifi.sta_password) {
                self.set_error(message);
                return;
            }
        }

        if matches!(wifi.mode, WiFiMode::WifiAp) {
            if let Err(message) = validate_sta_field("AP SSID", &wifi.ap_ssid) {
                self.set_error(message);
                return;
            }
            if let Err(message) = validate_sta_field("AP password", &wifi.ap_password) {
                self.set_error(message);
                return;
            }
        }

        let channel = if wifi.mode.allows_channel() {
            let channel = parse_optional_u16(&wifi.channel);
            if !wifi.channel.trim().is_empty() && channel.is_none() {
                self.set_error("Wi-Fi channel must be a valid number");
                return;
            }
            channel
        } else {
            None
        };

        let mut body = json!({ "mode": wifi.mode.as_api_value() });

        if matches!(wifi.mode, WiFiMode::Station) {
            if let Some(v) = empty_to_none(&wifi.sta_ssid) {
                body["sta_ssid"] = json!(v);
            }
            if let Some(v) = empty_to_none(&wifi.sta_password) {
                body["sta_password"] = json!(v);
            }
        }

        if matches!(wifi.mode, WiFiMode::WifiAp) {
            body["ap_ssid"] = json!(wifi.ap_ssid.trim());
            body["ap_password"] = json!(wifi.ap_password.trim());
            body["ap_dhcp"] = json!(wifi.ap_dhcp);
        }

        if let Some(ch) = channel {
            body["channel"] = json!(ch);
        }

        if wifi.mode.is_esp_now() {
            let peer_mac = wifi.peer_mac.trim();
            if !peer_mac.is_empty() {
                if let Err(message) = validate_peer_mac(peer_mac) {
                    self.set_error(message);
                    return;
                }
            }
            body["peer_mac"] = json!(peer_mac);
            body["ht40"] = json!(wifi.ht40.as_api_value());
        }

        // Keep the stored form in sync with what was submitted — a no-op for
        // UI-driven edits (the form was the source), but required for the
        // pairing-preset queue, whose later SetTraffic step consults
        // forms.wifi.mode before the async config re-fetch lands.
        if let Some(device) = self.state.device_mut_by_id(id) {
            device.forms.wifi = wifi.clone();
        }
        self.submit_device_post(id, "set_wifi", "config/wifi", Some(body));
    }

    fn apply_pairing_preset(
        &mut self,
        preset: PairingPreset,
        device_ids: [String; 2],
        channel: u8,
    ) {
        if preset.requires_v07() {
            for id in &device_ids {
                let Some(device) = self.state.devices.iter().find(|d| d.id == *id) else {
                    self.set_error(format!("Device not found: {id}"));
                    return;
                };
                if device.firmware_verified != Some(true) {
                    self.set_error(format!("Device {id} firmware not verified"));
                    return;
                }
                if !device.supports_v07_modes() {
                    self.set_error(format!(
                        "Preset '{}' requires esp-csi-cli-rs ≥ 0.7.0 on both boards",
                        preset.label()
                    ));
                    return;
                }
            }
        }

        let ch = channel.to_string();
        let ap_ssid = "esp-csi-ap".to_owned();

        let (wifi_a, wifi_b, proto_a, proto_b, traffic_a, traffic_b) = match preset {
            PairingPreset::SoftApLab => (
                WiFiForm {
                    mode: WiFiMode::WifiAp,
                    ap_ssid: ap_ssid.clone(),
                    channel: ch.clone(),
                    ..WiFiForm::default()
                },
                WiFiForm {
                    mode: WiFiMode::Station,
                    sta_ssid: ap_ssid,
                    ..WiFiForm::default()
                },
                Some(WifiProtocol::N),
                Some(WifiProtocol::N),
                // One-directional lab topology (hardware-verified): the AP
                // floods unsolicited echo replies at a serial-sustainable
                // 1000 Hz; the station is receive-only (0 = flood TX task
                // off). Both boards flooding at once contend for airtime and
                // collapse the delivered CSI rate.
                Some(TrafficForm {
                    frequency_hz: "1000".to_owned(),
                    unsolicited: true,
                }),
                Some(TrafficForm {
                    frequency_hz: "0".to_owned(),
                    unsolicited: false,
                }),
            ),
            PairingPreset::EspNowFastSimplex => (
                WiFiForm {
                    mode: WiFiMode::EspNowFastCollector,
                    channel: ch.clone(),
                    ..WiFiForm::default()
                },
                WiFiForm {
                    mode: WiFiMode::EspNowFastSource,
                    channel: ch,
                    ..WiFiForm::default()
                },
                None,
                None,
                None,
                None,
            ),
            PairingPreset::EspNowBalanced => (
                WiFiForm {
                    mode: WiFiMode::EspNowCentral,
                    channel: ch.clone(),
                    ..WiFiForm::default()
                },
                WiFiForm {
                    mode: WiFiMode::EspNowPeripheral,
                    channel: ch,
                    ..WiFiForm::default()
                },
                None,
                None,
                None,
                None,
            ),
        };

        let mut queue = Vec::new();
        let push_device = |queue: &mut Vec<(String, DeviceAction)>,
                           id: &str,
                           wifi: WiFiForm,
                           protocol: Option<WifiProtocol>,
                           traffic: Option<TrafficForm>| {
            queue.push((id.to_owned(), DeviceAction::ResetConfig));
            queue.push((id.to_owned(), DeviceAction::SetWifi(wifi)));
            if let Some(p) = protocol {
                queue.push((id.to_owned(), DeviceAction::SetProtocol(p)));
            }
            if let Some(t) = traffic {
                queue.push((id.to_owned(), DeviceAction::SetTraffic(t)));
            }
        };

        push_device(
            &mut queue,
            &device_ids[0],
            wifi_a,
            proto_a,
            traffic_a,
        );
        push_device(
            &mut queue,
            &device_ids[1],
            wifi_b,
            proto_b,
            traffic_b,
        );

        self.state.push_event(format!(
            "Applying preset '{}' to {} and {}",
            preset.label(),
            device_ids[0],
            device_ids[1]
        ));
        self.preset_queue = queue;
        self.run_next_preset_step();
    }

    fn run_next_preset_step(&mut self) {
        if let Some((id, action)) = self.preset_queue.first().cloned() {
            self.process_device_action(id, action);
        }
    }

    fn advance_preset_queue(&mut self, success: bool) {
        if self.preset_queue.is_empty() {
            return;
        }
        if success {
            self.preset_queue.remove(0);
            if self.preset_queue.is_empty() {
                self.state
                    .push_event("Pairing preset applied to both devices".to_owned());
                self.state.transient.status_message = "Pairing preset complete".to_owned();
            } else {
                self.run_next_preset_step();
            }
        } else {
            self.preset_queue.clear();
            self.state.push_event("Pairing preset aborted due to error".to_owned());
        }
    }

    /// Push a device-discovery poll if the interval has elapsed.
    fn maybe_poll_devices(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let due = match self.last_devices_poll {
            None => true,
            Some(last) => now - last >= DEVICE_POLL_INTERVAL_SECS,
        };
        if due {
            self.last_devices_poll = Some(now);
            self.state.push_intent(UserIntent::FetchDevices);
        }
    }

    /// Poll and apply core worker events without blocking the frame loop.
    fn process_core_events(&mut self) {
        let mut followups: Vec<UserIntent> = Vec::new();
        while let Some(event) = self.core.try_recv() {
            match event {
                CoreEvent::ApiResponse(response) => {
                    self.handle_api_response(response, &mut followups);
                }
                CoreEvent::WebSocketConnected { device_id } => {
                    if let Some(device) = self.state.device_mut_by_id(&device_id) {
                        device.ws_connected = true;
                    }
                    self.state.transient.status_message =
                        format!("WebSocket connected ({device_id})");
                    self.state.transient.error_message.clear();
                    self.state.push_event(format!("[{device_id}] WebSocket connected"));
                }
                CoreEvent::WebSocketDisconnected { device_id, reason } => {
                    if let Some(device) = self.state.device_mut_by_id(&device_id) {
                        device.ws_connected = false;
                    }
                    // No more frames will arrive — finalize any recording so the
                    // Parquet footer is written and the file is readable.
                    self.finalize_recording(&device_id);
                    self.state
                        .push_event(format!("[{device_id}] WebSocket disconnected: {reason}"));
                    // A close may mean the device was unplugged — re-discover.
                    followups.push(UserIntent::FetchDevices);
                }
                CoreEvent::WebSocketFrame { device_id, bytes } => {
                    if let Some(recorder) = self.recorders.get_mut(&device_id) {
                        let now = chrono::Utc::now().timestamp_micros();
                        let _ = recorder.record_frame(&bytes, now);
                        if let Some(device) = self.state.device_mut_by_id(&device_id) {
                            device.recorded_frames = recorder.frames_written;
                            device.record_decode_errors = recorder.decode_errors;
                        }
                    }
                    if let Some(device) = self.state.device_mut_by_id(&device_id) {
                        device.push_frame(&bytes);
                    }
                }
                CoreEvent::Log(line) => {
                    self.state.push_event(line);
                }
            }
        }
        for followup in followups {
            self.state.push_intent(followup);
        }
    }

    /// Handle one HTTP response, routing per-device payloads to the device.
    fn handle_api_response(&mut self, response: ApiResponseEvent, followups: &mut Vec<UserIntent>) {
        if response.label == "fetch_devices" {
            self.handle_device_list(response, followups);
            return;
        }

        let device_label = response.device_id.clone().unwrap_or_default();

        if response.success {
            self.state.transient.status_message = format!(
                "[{}] {} (HTTP {}): {}",
                device_label, response.label, response.status, response.message
            );
            self.state.transient.error_message.clear();
        } else {
            self.state.transient.error_message =
                format_error(&response.label, response.status, &response.message);
        }

        self.state.push_event(format!(
            "[{}] {} -> HTTP {}: {}",
            device_label, response.label, response.status, response.message
        ));

        // A 404 on a per-device route means the device went away — re-discover.
        if response.status == 404 {
            followups.push(UserIntent::FetchDevices);
        }

        let Some(id) = response.device_id.clone() else {
            return;
        };

        if response.success {
            if let Some(device) = self.state.device_mut_by_id(&id) {
                match response.label.as_str() {
                    "fetch_config" => {
                        if let Some(config) =
                            response.data.clone().and_then(parse_envelope::<DeviceConfig>)
                        {
                            let applied = device.apply_device_config(config);
                            if applied == 0 && !device.auto_resetting_cache {
                                device.auto_resetting_cache = true;
                                followups.push(UserIntent::Device {
                                    id: id.clone(),
                                    action: DeviceAction::ResetConfig,
                                });
                            } else {
                                device.auto_resetting_cache = false;
                            }
                        } else {
                            device.auto_resetting_cache = false;
                        }
                    }
                    "fetch_info" => {
                        if let Some(info) =
                            response.data.clone().and_then(parse_envelope::<DeviceInfo>)
                        {
                            device.firmware_verified = Some(true);
                            device.latest_info = Some(info);
                        }
                    }
                    "fetch_status" => {
                        if let Some(status) =
                            response.data.clone().and_then(parse_envelope::<ControlStatus>)
                        {
                            device.apply_control_status(status);
                        }
                    }
                    "start_collection" => device.collection_running = Some(true),
                    "stop_collection" => device.collection_running = Some(false),
                    "reset_device" => {
                        device.collection_running = Some(false);
                        device.firmware_verified = None;
                        device.latest_info = None;
                    }
                    // Any successful config-mutating POST repopulates a slot in the
                    // server cache, so re-pull `config` to keep the form in sync.
                    "reset_config" | "set_wifi" | "set_traffic" | "set_csi" | "set_csi_preset"
                    | "set_collection_mode" | "set_output_mode" | "set_protocol"
                    | "set_rate" | "set_io_tasks" | "set_csi_delivery" => {
                        followups.push(UserIntent::Device {
                            id: id.clone(),
                            action: DeviceAction::FetchConfig,
                        });
                    }
                    _ => {}
                }
            }
        } else if response.label == "fetch_info" && response.status != 0 {
            if let Some(device) = self.state.device_mut_by_id(&id) {
                device.firmware_verified = Some(false);
            }
        } else if response.label == "reset_config" {
            if let Some(device) = self.state.device_mut_by_id(&id) {
                device.auto_resetting_cache = false;
            }
        }

        self.maybe_advance_preset_queue(&response);
    }

    fn maybe_advance_preset_queue(&mut self, response: &ApiResponseEvent) {
        if self.preset_queue.is_empty() {
            return;
        }
        let Some(resp_id) = response.device_id.as_deref() else {
            return;
        };
        if !self.preset_queue.iter().any(|(id, _)| id == resp_id) {
            return;
        }
        if !response.success {
            self.advance_preset_queue(false);
            return;
        }
        let Some((head_id, head_action)) = self.preset_queue.first() else {
            return;
        };
        if head_id != resp_id {
            return;
        }
        if response.label == preset_action_label(head_action) {
            self.advance_preset_queue(true);
        }
    }

    /// Reconcile a `GET /api/devices` payload and load details for new devices.
    fn handle_device_list(&mut self, response: ApiResponseEvent, followups: &mut Vec<UserIntent>) {
        if !response.success {
            self.state.transient.server_status = ServerStatus::Disconnected;
            self.state.transient.error_message =
                format_error("fetch_devices", response.status, &response.message);
            return;
        }

        self.state.transient.server_status = ServerStatus::Connected;

        let Some(entries) = response.data.and_then(parse_device_list) else {
            return;
        };

        let outcome = self.state.reconcile_devices(entries);

        for id in &outcome.removed_ids {
            self.core.submit(CoreCommand::DisconnectWebSocket {
                device_id: id.clone(),
            });
            self.finalize_recording(id);
            self.state.push_event(format!("[{id}] device removed"));
        }

        for id in &outcome.new_ids {
            if let Some(device) = self.state.device_mut_by_id(id) {
                device.details_loaded = true;
            }
            followups.push(UserIntent::Device {
                id: id.clone(),
                action: DeviceAction::FetchInfo,
            });
            followups.push(UserIntent::Device {
                id: id.clone(),
                action: DeviceAction::FetchConfig,
            });
            followups.push(UserIntent::Device {
                id: id.clone(),
                action: DeviceAction::FetchStatus,
            });
        }

        if outcome.changed {
            let count = self.state.devices.len();
            self.state.transient.status_message = format!("{count} device(s) attached");
            self.state
                .push_event(format!("Device set changed: {count} attached"));
        }
    }

    /// Apply device-selection intents immediately (keeps the selector popup responsive).
    fn apply_selector_intent(&mut self, intent: UserIntent) {
        match intent {
            UserIntent::ToggleDeviceSelection(id) => self.state.toggle_selection(id),
            UserIntent::SelectAllDevices => {
                self.state.selected_device_ids = self.device_ids();
            }
            UserIntent::ClearDeviceSelection => {
                self.state.selected_device_ids.clear();
            }
            other => self.state.push_intent(other),
        }
    }

    /// Render the shared top panel (host/port, device selector, tabs, status).
    fn render_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Host");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.server_host).desired_width(140.0),
                );
                ui.label("Port");
                ui.add(
                    egui::TextEdit::singleline(&mut self.state.server_port).desired_width(60.0),
                );

                if ui.button("Connect").clicked() {
                    // Fire discovery immediately against the freshly-typed
                    // address rather than waiting for the next poll tick.
                    self.state.transient.server_status = ServerStatus::Connecting;
                    self.state.push_intent(UserIntent::FetchDevices);
                }

                server_status_indicator(ui, self.state.transient.server_status);

                ui.separator();
                ui.label("Devices");

                let selected_text = ui::selector::selection_label(&self.state);
                let mut selector_intents: Vec<UserIntent> = Vec::new();
                egui::ComboBox::from_id_salt("device_selector")
                    .selected_text(selected_text)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        ui::selector::render_popup(ui, &self.state, &mut selector_intents);
                    });
                for intent in selector_intents {
                    self.apply_selector_intent(intent);
                }

                if ui.button("Refresh Devices").clicked() {
                    self.state.push_intent(UserIntent::FetchDevices);
                }
            });

            let selected = self.state.selected_device_ids.clone();
            ui.horizontal_wrapped(|ui| {
                let has_selection = !selected.is_empty();
                if ui.add_enabled(has_selection, egui::Button::new("Fetch Info")).clicked() {
                    for id in &selected {
                        self.state.push_device_action(id.clone(), DeviceAction::FetchInfo);
                    }
                }
                if ui.add_enabled(has_selection, egui::Button::new("Fetch Config")).clicked() {
                    for id in &selected {
                        self.state.push_device_action(id.clone(), DeviceAction::FetchConfig);
                    }
                }
                if ui.add_enabled(has_selection, egui::Button::new("Fetch Status")).clicked() {
                    for id in &selected {
                        self.state.push_device_action(id.clone(), DeviceAction::FetchStatus);
                    }
                }
            });

            ui.horizontal_wrapped(|ui| {
                tab_button(ui, &mut self.state, Tab::Devices, "Devices");
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
    /// 2. poll device discovery on an interval
    /// 3. process queued user intents
    /// 4. render UI panels (buffering UI-issued actions, then enqueuing them)
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_core_events();
        self.maybe_poll_devices(ctx);
        self.process_intents();

        self.render_top_bar(ctx);

        let active_tab = self.state.transient.active_tab;
        let mut intents: Vec<UserIntent> = Vec::new();
        // (device_id, action) pairs collected from the per-device columns, routed
        // back to the right device after rendering.
        let mut tagged_actions: Vec<(String, DeviceAction)> = Vec::new();
        // Lift `export_dir` out so the Stream view can edit it without a second
        // mutable borrow of `self.state` alongside the selected devices.
        let mut export_dir = std::mem::take(&mut self.state.export_dir);

        egui::CentralPanel::default().show(ctx, |ui| match active_tab {
            Tab::Devices => ui::devices::render(ui, &mut self.state, &mut intents),
            _ => {
                let selected_ids = self.state.selected_device_ids.clone();
                if selected_ids.is_empty() {
                    ui.heading("No device selected");
                    ui.label("Select one or more devices from the Devices tab or the top bar.");
                    return;
                }

                if matches!(active_tab, Tab::Control | Tab::Stream) {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!("{} selected", selected_ids.len()));
                        ui.separator();
                        if ui.button("Start Selected").clicked() {
                            intents.push(UserIntent::StartSelectedCollections {
                                duration_seconds: String::new(),
                            });
                        }
                        if ui.button("Stop Selected").clicked() {
                            intents.push(UserIntent::StopSelectedCollections);
                        }
                        if active_tab == Tab::Stream {
                            ui.separator();
                            if ui.button("Start Recording (Selected)").clicked() {
                                intents.push(UserIntent::StartSelectedRecording);
                            }
                            if ui.button("Stop Recording (Selected)").clicked() {
                                intents.push(UserIntent::StopSelectedRecording);
                            }
                        }
                    });
                    ui.separator();
                }

                if active_tab == Tab::Stream {
                    ui::stream::render_export_dir(ui, &mut export_dir);
                    ui.separator();
                }

                ui::detail::render(
                    ui,
                    active_tab,
                    &mut self.state,
                    &selected_ids,
                    &mut tagged_actions,
                );
            }
        });

        self.state.export_dir = export_dir;

        for intent in intents {
            self.state.push_intent(intent);
        }
        for (id, action) in tagged_actions {
            self.state.push_device_action(id, action);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Render a colored "● <label>" badge reflecting the server connection state.
fn server_status_indicator(ui: &mut egui::Ui, status: ServerStatus) {
    let color = match status {
        ServerStatus::Connected => egui::Color32::from_rgb(60, 180, 75),
        ServerStatus::Connecting => egui::Color32::from_rgb(230, 180, 40),
        ServerStatus::Disconnected => egui::Color32::from_rgb(220, 80, 80),
        ServerStatus::Unknown => egui::Color32::GRAY,
    };
    ui.colored_label(color, format!("● {}", status.label()));
}

/// Parse the `GET /api/devices` payload from a bare array or `data` envelope.
fn parse_device_list(data: Value) -> Option<Vec<DeviceListEntry>> {
    if let Ok(list) = serde_json::from_value::<Vec<DeviceListEntry>>(data.clone()) {
        return Some(list);
    }
    if let Some(inner) = data.get("data") {
        return serde_json::from_value::<Vec<DeviceListEntry>>(inner.clone()).ok();
    }
    None
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

/// HTTP label emitted by [`process_device_action`] for preset queue steps.
fn preset_action_label(action: &DeviceAction) -> &str {
    match action {
        DeviceAction::ResetConfig => "reset_config",
        DeviceAction::SetWifi(_) => "set_wifi",
        DeviceAction::SetTraffic(_) => "set_traffic",
        DeviceAction::SetProtocol(_) => "set_protocol",
        _ => "",
    }
}

/// Validate an ESP-NOW peer MAC (`aa:bb:cc:dd:ee:ff` or `aa-bb-...`).
fn validate_peer_mac(mac: &str) -> Result<(), String> {
    let sep = if mac.contains(':') {
        ':'
    } else if mac.contains('-') {
        '-'
    } else {
        return Err("Peer MAC must use ':' or '-' separators (aa:bb:cc:dd:ee:ff)".to_owned());
    };
    let octets: Vec<&str> = mac.split(sep).collect();
    if octets.len() != 6
        || octets
            .iter()
            .any(|o| o.len() != 2 || !o.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(format!("Invalid peer MAC '{mac}' (use aa:bb:cc:dd:ee:ff)"));
    }
    Ok(())
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
        404 => Some("device not found — it may have been unplugged"),
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
