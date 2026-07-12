use crate::state::{AppState, DeviceAction, PairingPreset, UserIntent};

/// Render the fleet overview: every attached device, per-device collection
/// control, fleet-wide Start/Stop All, pairing presets, and the global event log.
pub fn render(ui: &mut egui::Ui, state: &mut AppState, intents: &mut Vec<UserIntent>) {
    ui.heading("Devices");
    ui.separator();

    ui.horizontal_wrapped(|ui| {
        if ui.button("Refresh Devices").clicked() {
            intents.push(UserIntent::FetchDevices);
        }
        ui.separator();
        if ui.button("Select All").clicked() {
            intents.push(UserIntent::SelectAllDevices);
        }
        if ui.button("Clear Selection").clicked() {
            intents.push(UserIntent::ClearDeviceSelection);
        }
        ui.separator();
        if ui.button("Start All").clicked() {
            intents.push(UserIntent::StartAllCollections {
                duration_seconds: String::new(),
            });
        }
        if ui.button("Stop All").clicked() {
            intents.push(UserIntent::StopAllCollections);
        }
        ui.separator();
        let has_selection = !state.selected_device_ids.is_empty();
        if ui
            .add_enabled(has_selection, egui::Button::new("Start Selected"))
            .clicked()
        {
            intents.push(UserIntent::StartSelectedCollections {
                duration_seconds: String::new(),
            });
        }
        if ui
            .add_enabled(has_selection, egui::Button::new("Stop Selected"))
            .clicked()
        {
            intents.push(UserIntent::StopSelectedCollections);
        }
    });

    render_pairing_presets(ui, state, intents);

    ui.separator();

    if state.devices.is_empty() {
        ui.label("No devices attached. Plug in an ESP32 — discovery polls every ~2s.");
    } else {
        let content_width = ui.available_width();
        let table_height = (ui.available_height() * 0.55).max(120.0);
        egui::ScrollArea::vertical()
            .id_salt("devices_table_scroll")
            .auto_shrink([false, false])
            .max_height(table_height)
            .show(ui, |ui| {
                ui.set_width(content_width);
                render_table(ui, state, intents);
            });
    }

    ui.separator();
    ui.label("Recent events");
    egui::ScrollArea::vertical()
        .id_salt("devices_events_scroll")
        .auto_shrink([false, false])
        .max_height(ui.available_height().max(80.0))
        .show(ui, |ui| {
            for line in state.events.iter().rev().take(80) {
                ui.add(egui::Label::new(line).wrap());
            }
        });
}

/// Fixed width for the device id cell (checkbox + MAC / alias).
const ID_COL_WIDTH: f32 = 156.0;

fn render_table(ui: &mut egui::Ui, state: &AppState, intents: &mut Vec<UserIntent>) {
    egui::Grid::new("devices_grid")
        .num_columns(8)
        .striped(true)
        .spacing([10.0, 4.0])
        .min_col_width(36.0)
        .show(ui, |ui| {
            ui.strong("Device");
            ui.strong("Port");
            ui.strong("Serial");
            ui.strong("Firmware");
            ui.strong("Collection");
            ui.strong("WS");
            ui.strong("Frames");
            ui.strong("Actions");
            ui.end_row();

            for device in &state.devices {
                render_device_cell(ui, state, device, intents);

                ui.label(port_basename(device.port_path.as_deref()));
                ui.label(tristate(device.serial_connected, "up", "down"));
                if let Some(fault) = &device.fault {
                    ui.colored_label(egui::Color32::RED, "FAULT")
                        .on_hover_text(fault);
                } else {
                    ui.label(tristate(device.firmware_verified, "ok", "no"));
                }
                ui.label(tristate(device.collection_running, "running", "idle"));
                ui.label(if device.ws_connected { "on" } else { "off" });
                ui.label(device.frames_received.to_string());

                ui.push_id(format!("actions_{}", device.id), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Start").clicked() {
                            intents.push(UserIntent::Device {
                                id: device.id.clone(),
                                action: DeviceAction::StartCollection {
                                    duration_seconds: device.forms.start_duration_seconds.clone(),
                                },
                            });
                        }
                        if ui.button("Stop").clicked() {
                            intents.push(UserIntent::Device {
                                id: device.id.clone(),
                                action: DeviceAction::StopCollection,
                            });
                        }
                    });
                });

                ui.end_row();
            }
        });

    // Full-width fault banners — the FAULT cell above is easy to miss and its
    // hover text invisible until pointed at; a wedged chip needs a human
    // action (reset button / replug), so spell out the recovery per device.
    for device in &state.devices {
        if let Some(fault) = &device.fault {
            ui.add_space(6.0);
            ui.colored_label(
                egui::Color32::RED,
                format!("⚠ {}: {}", device.id, fault),
            );
        }
    }
}

/// Checkbox + device id in one grid cell, width-capped so the row stays aligned.
fn render_device_cell(
    ui: &mut egui::Ui,
    state: &AppState,
    device: &crate::state::DeviceState,
    intents: &mut Vec<UserIntent>,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(ID_COL_WIDTH, 0.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_max_width(ID_COL_WIDTH);

            let mut selected = state.is_selected(&device.id);
            if ui.checkbox(&mut selected, "").changed() {
                intents.push(UserIntent::ToggleDeviceSelection(device.id.clone()));
            }

            let id_text = if let Some(mac) = &device.mac {
                format!("{}\n{}", device.id, mac)
            } else {
                device.id.clone()
            };
            let id_label = egui::Label::new(id_text)
                .wrap()
                .selectable(selected)
                .sense(egui::Sense::click());
            let response = ui.add(id_label).on_hover_text(&device.id);
            if response.clicked() {
                intents.push(UserIntent::ToggleDeviceSelection(device.id.clone()));
            }
        },
    );
}

fn port_basename(port: Option<&str>) -> String {
    port.map(|p| {
        p.rsplit(['/', '\\'])
            .next()
            .unwrap_or(p)
            .to_owned()
    })
    .unwrap_or_else(|| "?".to_owned())
}

fn tristate(value: Option<bool>, true_label: &str, false_label: &str) -> String {
    match value {
        Some(true) => true_label.to_owned(),
        Some(false) => false_label.to_owned(),
        None => "?".to_owned(),
    }
}

fn render_pairing_presets(
    ui: &mut egui::Ui,
    state: &mut AppState,
    intents: &mut Vec<UserIntent>,
) {
    if state.selected_device_ids.len() != 2 {
        return;
    }

    ui.collapsing("Pairing presets (2 devices selected)", |ui| {
        ui.add(
            egui::Label::new(format!(
                "Device 1: {} — Device 2: {}",
                state.selected_device_ids[0], state.selected_device_ids[1]
            ))
            .wrap(),
        );
        ui.horizontal(|ui| {
            ui.label("Channel");
            ui.add(
                egui::TextEdit::singleline(&mut state.transient.preset_channel)
                    .desired_width(48.0),
            );
        });
        ui.add_space(4.0);

        let channel = state.transient.preset_channel.trim().parse::<u8>().unwrap_or(6);
        let ids = [
            state.selected_device_ids[0].clone(),
            state.selected_device_ids[1].clone(),
        ];

        ui.horizontal_wrapped(|ui| {
            for preset in [
                PairingPreset::SoftApLab,
                PairingPreset::EspNowFastSimplex,
                PairingPreset::EspNowBalanced,
            ] {
                if ui.button(preset.label()).clicked() {
                    intents.push(UserIntent::Device {
                        id: ids[0].clone(),
                        action: DeviceAction::ApplyPairingPreset {
                            preset,
                            device_ids: ids.clone(),
                            channel,
                        },
                    });
                }
            }
        });
        ui.add(
            egui::Label::new(
                "Applies reset + Wi-Fi (+ protocol/traffic where needed) to both boards in \
                 order. SoftAP lab pair: device 1 becomes the AP flooding one-directional \
                 (unsolicited echo-reply) traffic at 1000 Hz; device 2 becomes a \
                 receive-only station — collect CSI on device 2.",
            )
            .wrap(),
        );
    });
}
