use crate::state::AppState;

/// Render the stream inspection view.
pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Stream");
    ui.separator();

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.transient.auto_scroll_stream, "Auto-scroll");
        ui.label(format!("Frames: {}", state.runtime.frames_received));
        ui.label(format!("Bytes: {}", state.runtime.bytes_received));
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .stick_to_bottom(state.transient.auto_scroll_stream)
        .show(ui, |ui| {
            egui::Grid::new("stream_frames_grid")
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Time");
                    ui.strong("Length");
                    ui.strong("Preview (hex)");
                    ui.end_row();

                    for frame in state.runtime.recent_frames.iter().rev().take(250) {
                        ui.label(&frame.timestamp);
                        ui.label(frame.length.to_string());
                        ui.monospace(&frame.preview_hex);
                        ui.end_row();
                    }
                });
        });
}
