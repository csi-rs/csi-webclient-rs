use csi_webclient::app;

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();
    options.viewport = options
        .viewport
        .clone()
        .with_inner_size([1024.0, 720.0])
        .with_min_inner_size([560.0, 420.0]);

    #[cfg(target_os = "macos")]
    {
        // Work around a macOS AppKit shutdown crash in the run_on_demand path.
        options.run_and_return = false;
    }

    eframe::run_native(
        "CSI Webserver Client",
        options,
        Box::new(|cc| Ok(Box::new(app::CsiClientApp::new(cc)))),
    )
}
