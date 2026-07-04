//! Multi-device detail layout: all tabs stack devices vertically at full panel width.

use crate::state::{AppState, DeviceAction, Tab};

use super::{config, control, dashboard, stream};

/// Space between vertically stacked device panels.
const STACK_DEVICE_GAP: f32 = 24.0;

/// Render the active detail tab for every selected device.
pub fn render(
    ui: &mut egui::Ui,
    active_tab: Tab,
    state: &mut AppState,
    selected_ids: &[String],
    actions: &mut Vec<(String, DeviceAction)>,
) {
    if selected_ids.is_empty() || matches!(active_tab, Tab::Devices) {
        return;
    }

    let content_width = ui.available_width();
    let scroll_height = ui.available_height();
    let scroll_id = match active_tab {
        Tab::Config => "config_devices_vscroll",
        Tab::Dashboard => "dashboard_devices_vscroll",
        Tab::Control => "control_devices_vscroll",
        Tab::Stream => "stream_devices_vscroll",
        Tab::Devices => return,
    };

    egui::ScrollArea::vertical()
        .id_salt(scroll_id)
        .auto_shrink([false, false])
        .max_height(scroll_height.max(0.0))
        .show(ui, |ui| {
            ui.set_width(content_width);
            for id in selected_ids {
                let device_id = id.clone();
                ui.push_id(&device_id, |ui| {
                    ui.set_max_width(content_width);
                    egui::Frame::group(ui.style())
                        .inner_margin(14.0)
                        .outer_margin(egui::Margin::symmetric(0, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let Some(device) = state.device_mut_by_id(&device_id) else {
                                return;
                            };
                            let mut acts = Vec::new();
                            match active_tab {
                                Tab::Config => config::render(ui, device, &mut acts),
                                Tab::Dashboard => dashboard::render(ui, device),
                                Tab::Control => control::render(ui, device, &mut acts),
                                Tab::Stream => stream::render(ui, device, &mut acts),
                                Tab::Devices => {}
                            }
                            for action in acts {
                                actions.push((device_id.clone(), action));
                            }
                        });
                    ui.add_space(STACK_DEVICE_GAP);
                });
            }
        });
}
