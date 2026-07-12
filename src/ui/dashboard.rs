use crate::state::DeviceState;

/// Render the dashboard view for one device.
pub fn render(ui: &mut egui::Ui, device: &DeviceState) {
    ui.add(
        egui::Label::new(format!("Dashboard — {}", device.id))
            .wrap(),
    );
    ui.add_space(10.0);

    ui.strong("Connection");
    ui.add_space(4.0);
    stat_row(ui, "Serial", format_tristate(device.serial_connected, "connected", "disconnected"));
    if let Some(mac) = &device.mac {
        stat_row(ui, "MAC", mac.clone());
    }
    stat_row(ui, "Port", device.port_path.clone().unwrap_or_else(|| "?".to_owned()));
    if let Some(baud) = device.baud_rate {
        stat_row(ui, "Baud", baud.to_string());
    }
    stat_row(
        ui,
        "Firmware",
        format_tristate(device.firmware_verified, "verified", "unverified"),
    );
    stat_row(
        ui,
        "Collection",
        format_tristate(device.collection_running, "running", "idle"),
    );

    if let Some(info) = &device.latest_info {
        ui.add_space(10.0);
        ui.strong("Firmware info");
        ui.add_space(4.0);
        stat_row(ui, "Name", info.name.clone().unwrap_or_default());
        stat_row(ui, "Version", info.version.clone().unwrap_or_default());
        stat_row(ui, "Chip", info.chip.clone().unwrap_or_default());
        stat_row(
            ui,
            "Protocol",
            info.protocol
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".to_owned()),
        );
        stat_row(ui, "Features", info.features.join(", "));
    }

    ui.add_space(10.0);
    ui.strong("Session");
    ui.add_space(4.0);
    stat_row(
        ui,
        "Collection role",
        device.forms.collection_mode.as_api_value().to_owned(),
    );
    stat_row(
        ui,
        "Output mode",
        device.forms.output_mode.as_api_value().to_owned(),
    );

    ui.add_space(10.0);
    ui.strong("Stream");
    ui.add_space(4.0);
    stat_row(
        ui,
        "WebSocket",
        if device.ws_connected {
            "connected".to_owned()
        } else {
            "disconnected".to_owned()
        },
    );
    stat_row(ui, "Frames", device.frames_received.to_string());
    stat_row(ui, "Bytes", device.bytes_received.to_string());
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(format!("{label}:"));
        ui.add_space(8.0);
        ui.add(egui::Label::new(value).wrap());
    });
    ui.add_space(2.0);
}

fn format_tristate(value: Option<bool>, true_label: &str, false_label: &str) -> String {
    match value {
        Some(true) => true_label.to_owned(),
        Some(false) => false_label.to_owned(),
        None => "?".to_owned(),
    }
}
