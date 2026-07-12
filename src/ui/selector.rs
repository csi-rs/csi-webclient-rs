//! Device multi-select widgets shared by the top bar and Devices tab.

use crate::state::{AppState, DeviceState, UserIntent};

/// Summary label for the top-bar combo box.
pub fn selection_label(state: &AppState) -> String {
    match state.selected_device_ids.len() {
        0 => "(none)".to_owned(),
        1 => state.selected_device_ids[0].clone(),
        n => format!("{n} selected"),
    }
}

/// Combo-box popup: checkboxes + Select All / Clear.
pub fn render_popup(ui: &mut egui::Ui, state: &AppState, intents: &mut Vec<UserIntent>) {
    ui.horizontal(|ui| {
        if ui.button("Select All").clicked() {
            intents.push(UserIntent::SelectAllDevices);
        }
        if ui.button("Clear").clicked() {
            intents.push(UserIntent::ClearDeviceSelection);
        }
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("device_selector_popup_scroll")
        .auto_shrink([false, false])
        .max_height(240.0)
        .show(ui, |ui| {
            for device in &state.devices {
                let mut selected = state.is_selected(&device.id);
                let label = device_label(device);
                if ui.checkbox(&mut selected, label).changed() {
                    intents.push(UserIntent::ToggleDeviceSelection(device.id.clone()));
                }
            }
        });
}

/// Checkbox column in the Devices table.
pub fn render_row_checkbox(
    ui: &mut egui::Ui,
    state: &AppState,
    device_id: &str,
    intents: &mut Vec<UserIntent>,
) {
    let mut selected = state.is_selected(device_id);
    if ui.checkbox(&mut selected, "").changed() {
        intents.push(UserIntent::ToggleDeviceSelection(device_id.to_owned()));
    }
}

fn device_label(device: &DeviceState) -> String {
    match device.latest_info.as_ref().and_then(|i| i.chip.clone()) {
        Some(chip) => format!("{} ({chip})", device.id),
        None => device.id.clone(),
    }
}
