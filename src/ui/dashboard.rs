use crate::state::AppState;

/// Render the dashboard view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Dashboard");
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
    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
        for line in state.runtime.events.iter().rev().take(80) {
            ui.label(line);
        }
    });
}
