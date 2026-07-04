use crate::state::{DeviceAction, DeviceState};

/// Render the control view for one device.
pub fn render(ui: &mut egui::Ui, device: &mut DeviceState, actions: &mut Vec<DeviceAction>) {
    ui.add(
        egui::Label::new(format!("Control — {}", device.id))
            .wrap(),
    );
    ui.add_space(10.0);
    render_body(ui, device, actions);
}

fn render_body(ui: &mut egui::Ui, device: &mut DeviceState, actions: &mut Vec<DeviceAction>) {
    ui.strong("Collection");
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        ui.label("Duration (seconds, optional)");
        ui.add(
            egui::TextEdit::singleline(&mut device.forms.start_duration_seconds)
                .desired_width(80.0),
        );
    });
    ui.add_space(6.0);

    ui.horizontal_wrapped(|ui| {
        if ui.button("Start Collection").clicked() {
            actions.push(DeviceAction::StartCollection {
                duration_seconds: device.forms.start_duration_seconds.clone(),
            });
        }
        if ui.button("Stop Collection").clicked() {
            actions.push(DeviceAction::StopCollection);
        }
        if ui.button("Show Stats").clicked() {
            actions.push(DeviceAction::ShowStats);
        }
    });

    ui.add_space(10.0);
    ui.strong("Device");
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        if ui.button("Reset Device (RTS)").clicked() {
            actions.push(DeviceAction::ResetDevice);
        }
        if ui.button("Fetch Status").clicked() {
            actions.push(DeviceAction::FetchStatus);
        }
        if ui.button("Fetch Info").clicked() {
            actions.push(DeviceAction::FetchInfo);
        }
        if ui.button("Fetch Config").clicked() {
            actions.push(DeviceAction::FetchConfig);
        }
    });

    ui.add_space(10.0);
    ui.strong("WebSocket");
    ui.add_space(4.0);

    ui.horizontal_wrapped(|ui| {
        if !device.ws_connected {
            if ui.button("Connect WebSocket").clicked() {
                actions.push(DeviceAction::ConnectWebSocket);
            }
        } else if ui.button("Disconnect WebSocket").clicked() {
            actions.push(DeviceAction::DisconnectWebSocket);
        }
        if ui.button("Clear Stream Frames").clicked() {
            actions.push(DeviceAction::ClearFrames);
        }
    });

    ui.add_space(8.0);
    ui.add(
        egui::Label::new(
            "Stop sends the literal 'q' byte (graceful). Reset pulses RTS and re-verifies firmware.",
        )
        .wrap(),
    );
}
