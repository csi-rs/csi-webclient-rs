use crate::state::{DeviceAction, DeviceState};

/// Max height of the per-device frame list (keeps multi-device view usable).
const FRAME_LIST_HEIGHT: f32 = 220.0;

/// Render the stream inspection view for one device.
///
/// Recording start/stop are queued into `actions`. The shared export directory
/// is edited once at the top of the Stream tab (see [`render_export_dir`]).
pub fn render(ui: &mut egui::Ui, device: &mut DeviceState, actions: &mut Vec<DeviceAction>) {
    ui.add(
        egui::Label::new(format!("Stream — {}", device.id))
            .wrap(),
    );
    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut device.auto_scroll_stream, "Auto-scroll");
        ui.label(format!("Frames: {}", device.frames_received));
        ui.label(format!("Bytes: {}", device.bytes_received));
    });

    render_recording_controls(ui, device, actions);

    ui.add_space(8.0);
    ui.strong("Recent frames");
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("stream_frames_scroll")
        .auto_shrink([false, false])
        .max_height(FRAME_LIST_HEIGHT)
        .stick_to_bottom(device.auto_scroll_stream)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if device.recent_frames.is_empty() {
                ui.label("No frames yet — connect the WebSocket and start collection.");
                return;
            }

            ui.horizontal_wrapped(|ui| {
                ui.strong("Time");
                ui.label("·");
                ui.strong("Len");
                ui.label("·");
                ui.strong("Preview (hex)");
            });
            ui.separator();

            for frame in device.recent_frames.iter().rev().take(250) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&frame.timestamp);
                    ui.label(format!("{} B", frame.length));
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&frame.preview_hex).monospace(),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(2.0);
            }
        });
}

/// Shared Parquet output directory (shown once above all device panels).
pub fn render_export_dir(ui: &mut egui::Ui, export_dir: &mut String) {
    ui.strong("Export to Parquet");
    ui.horizontal_wrapped(|ui| {
        ui.label("Output dir");
        let field_w = (ui.available_width() - 80.0).clamp(120.0, 480.0);
        ui.add(egui::TextEdit::singleline(export_dir).desired_width(field_w));
    });
}

/// Per-device recording start/stop and status.
fn render_recording_controls(
    ui: &mut egui::Ui,
    device: &DeviceState,
    actions: &mut Vec<DeviceAction>,
) {
    ui.add_space(6.0);
    ui.strong("Recording");
    ui.add_space(4.0);

    let chip_known = device
        .latest_info
        .as_ref()
        .and_then(|i| i.chip.as_deref())
        .is_some();

    ui.horizontal_wrapped(|ui| {
        if device.recording {
            if ui.button("⏹ Stop Recording").clicked() {
                actions.push(DeviceAction::StopRecording);
            }
            ui.colored_label(
                egui::Color32::from_rgb(220, 80, 80),
                format!("● Recording — {} frame(s)", device.recorded_frames),
            );
            if device.record_decode_errors > 0 {
                ui.label(format!("({} undecodable)", device.record_decode_errors));
            }
        } else {
            let start = ui.add_enabled(chip_known, egui::Button::new("⏺ Start Recording"));
            if start.clicked() {
                actions.push(DeviceAction::StartRecording);
            }
            if !chip_known {
                ui.label("— chip unknown; Fetch Info first");
            } else if !device.ws_connected {
                ui.label("— connect WebSocket to capture");
            }
        }
    });

    if let Some(path) = &device.record_path {
        let label = if device.recording {
            format!("Writing: {path}")
        } else {
            format!("Saved: {path}")
        };
        ui.add(egui::Label::new(label).wrap());
    }
}
