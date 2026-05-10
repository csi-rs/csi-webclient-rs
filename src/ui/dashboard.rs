use crate::state::AppState;

/// Render the dashboard view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Dashboard");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        ui.strong("Serial:");
        ui.label(format_tristate(state.runtime.serial_connected, "connected", "disconnected"));
        ui.separator();

        ui.strong("Firmware:");
        ui.label(format_tristate(state.runtime.firmware_verified, "verified", "unverified"));
        ui.separator();

        ui.strong("Collection:");
        ui.label(format_tristate(state.runtime.collection_running, "running", "idle"));
        ui.separator();

        ui.strong("Port:");
        ui.label(state.runtime.port_path.clone().unwrap_or_else(|| "?".to_owned()));
    });

    ui.separator();

    if let Some(info) = &state.runtime.latest_info {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Name:");
            ui.label(info.name.clone().unwrap_or_default());
            ui.separator();

            ui.strong("Version:");
            ui.label(info.version.clone().unwrap_or_default());
            ui.separator();

            ui.strong("Chip:");
            ui.label(info.chip.clone().unwrap_or_default());
            ui.separator();

            ui.strong("Protocol:");
            ui.label(
                info.protocol
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_owned()),
            );
            ui.separator();

            ui.strong("Features:");
            ui.label(info.features.join(", "));
        });
        ui.separator();
    }

    ui.horizontal_wrapped(|ui| {
        ui.strong("Collection role:");
        ui.label(state.persistent.collection_mode.as_api_value());
        ui.separator();

        ui.strong("Output mode:");
        ui.label(state.persistent.output_mode.as_api_value());
        ui.separator();

        ui.strong("Log mode:");
        ui.label(state.persistent.log_mode.as_api_value());
    });

    ui.separator();

    ui.horizontal_wrapped(|ui| {
        ui.label(format!("HTTP Base: {}", state.base_http_url()));
        ui.separator();
        ui.label(format!(
            "WebSocket: {}",
            if state.runtime.ws_connected {
                "Connected"
            } else {
                "Disconnected"
            }
        ));
        ui.separator();
        ui.label(format!("Frames: {}", state.runtime.frames_received));
        ui.separator();
        ui.label(format!("Bytes: {}", state.runtime.bytes_received));
    });

    ui.separator();
    ui.label("Recent events");
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for line in state.runtime.events.iter().rev().take(80) {
                ui.add(egui::Label::new(line).wrap());
            }
        });
}

fn format_tristate(value: Option<bool>, true_label: &str, false_label: &str) -> String {
    match value {
        Some(true) => true_label.to_owned(),
        Some(false) => false_label.to_owned(),
        None => "?".to_owned(),
    }
}
