mod http;
pub mod messages;
mod ws;

use crate::core::messages::{CoreCommand, CoreEvent};
use std::sync::mpsc::{self, Receiver, Sender};

/// App-facing handle for the background core worker.
///
/// The handle is intentionally lightweight and only exposes:
///
/// - command submission (`submit`)
/// - non-blocking event polling (`try_recv`)
pub struct CoreHandle {
    cmd_tx: Sender<CoreCommand>,
    event_rx: Receiver<CoreEvent>,
}

impl CoreHandle {
    /// Spawn a new core worker thread and return a handle.
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<CoreCommand>();
        let (event_tx, event_rx) = mpsc::channel::<CoreEvent>();

        std::thread::Builder::new()
            .name("csi-core-worker".to_owned())
            .spawn(move || worker_loop(cmd_rx, event_tx))
            .expect("failed to spawn core worker thread");

        Self { cmd_tx, event_rx }
    }

    /// Submit a command to the core worker.
    pub fn submit(&self, command: CoreCommand) {
        let _ = self.cmd_tx.send(command);
    }

    /// Poll the next core event without blocking the UI thread.
    pub fn try_recv(&self) -> Option<CoreEvent> {
        self.event_rx.try_recv().ok()
    }
}

impl Drop for CoreHandle {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(CoreCommand::Shutdown);
    }
}

fn worker_loop(cmd_rx: Receiver<CoreCommand>, event_tx: Sender<CoreEvent>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            let _ = event_tx.send(CoreEvent::Log(format!(
                "Failed to initialize async runtime: {err}"
            )));
            return;
        }
    };

    let mut ws_stop_tx: Option<tokio::sync::oneshot::Sender<()>> = None;
    let mut ws_task: Option<tokio::task::JoinHandle<()>> = None;

    while let Ok(command) = cmd_rx.recv() {
        match command {
            CoreCommand::ExecuteApi(request) => {
                let event = runtime.block_on(http::execute_api_request(request));
                let _ = event_tx.send(event);
            }
            CoreCommand::ConnectWebSocket { url } => {
                stop_ws_task(&runtime, &mut ws_stop_tx, &mut ws_task);

                let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
                ws_stop_tx = Some(stop_tx);

                let event_tx_clone = event_tx.clone();
                ws_task = Some(runtime.spawn(async move {
                    ws::run_ws_loop(url, stop_rx, event_tx_clone).await;
                }));
            }
            CoreCommand::DisconnectWebSocket => {
                stop_ws_task(&runtime, &mut ws_stop_tx, &mut ws_task);
                let _ = event_tx.send(CoreEvent::WebSocketDisconnected {
                    reason: "Disconnected".to_owned(),
                });
            }
            CoreCommand::Shutdown => {
                stop_ws_task(&runtime, &mut ws_stop_tx, &mut ws_task);
                break;
            }
        }
    }
}

fn stop_ws_task(
    runtime: &tokio::runtime::Runtime,
    ws_stop_tx: &mut Option<tokio::sync::oneshot::Sender<()>>,
    ws_task: &mut Option<tokio::task::JoinHandle<()>>,
) {
    if let Some(stop_tx) = ws_stop_tx.take() {
        let _ = stop_tx.send(());
    }

    if let Some(task) = ws_task.take() {
        let _ = runtime.block_on(task);
    }
}
