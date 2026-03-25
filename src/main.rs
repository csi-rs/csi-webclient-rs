use csi_webserver_cli_rs::app;

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();

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
