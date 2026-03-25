use crate::state::{AppState, UserIntent};

/// Render the control view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Control");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Duration (seconds, optional)");
        ui.text_edit_singleline(&mut state.persistent.start_duration_seconds);
    });

    ui.horizontal(|ui| {
        if ui.button("Start Collection").clicked() {
            state.push_intent(UserIntent::StartCollection {
                duration_seconds: state.persistent.start_duration_seconds.clone(),
            });
        }

        if ui.button("Reset Device (RTS)").clicked() {
            state.push_intent(UserIntent::ResetDevice);
        }

        if ui.button("Fetch Config").clicked() {
            state.push_intent(UserIntent::FetchConfig);
        }
    });

    ui.separator();

    ui.horizontal(|ui| {
        if !state.runtime.ws_connected {
            if ui.button("Connect WebSocket").clicked() {
                state.push_intent(UserIntent::ConnectWebSocket);
            }
        } else if ui.button("Disconnect WebSocket").clicked() {
            state.push_intent(UserIntent::DisconnectWebSocket);
        }

        if ui.button("Clear Stream Frames").clicked() {
            state.push_intent(UserIntent::ClearFrames);
        }
    });
}
