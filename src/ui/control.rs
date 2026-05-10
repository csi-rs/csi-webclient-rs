use crate::state::{AppState, UserIntent};

/// Render the control view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Control");
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_body(ui, state);
        });
}

fn render_body(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Duration (seconds, optional)");
        ui.add(
            egui::TextEdit::singleline(&mut state.persistent.start_duration_seconds)
                .desired_width(80.0),
        );
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button("Start Collection").clicked() {
            state.push_intent(UserIntent::StartCollection {
                duration_seconds: state.persistent.start_duration_seconds.clone(),
            });
        }

        if ui.button("Stop Collection").clicked() {
            state.push_intent(UserIntent::StopCollection);
        }

        if ui.button("Show Stats").clicked() {
            state.push_intent(UserIntent::ShowStats);
        }
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button("Reset Device (RTS)").clicked() {
            state.push_intent(UserIntent::ResetDevice);
        }

        if ui.button("Fetch Status").clicked() {
            state.push_intent(UserIntent::FetchStatus);
        }

        if ui.button("Fetch Info").clicked() {
            state.push_intent(UserIntent::FetchInfo);
        }

        if ui.button("Fetch Config").clicked() {
            state.push_intent(UserIntent::FetchConfig);
        }
    });

    ui.separator();

    ui.horizontal_wrapped(|ui| {
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

    ui.separator();
    ui.add(
        egui::Label::new(
            "Stop sends the literal 'q' byte (graceful). Reset pulses RTS and re-verifies firmware.",
        )
        .wrap(),
    );
}
