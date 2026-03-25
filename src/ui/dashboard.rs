use crate::state::AppState;

/// Render the dashboard view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Dashboard");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        ui.strong("Collection role:");
        ui.label(state.persistent.collection_mode.as_api_value());
        ui.separator();

        ui.strong("Collection active:");
        ui.label(if state.runtime.collection_active_estimate {
            "yes (estimated)"
        } else {
            "no (estimated)"
        });
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
    let remaining_height = ui.available_height();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(remaining_height)
        .show(ui, |ui| {
        for line in state.runtime.events.iter().rev().take(80) {
            ui.label(line);
        }
        });
}
