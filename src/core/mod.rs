mod http;
pub mod messages;
mod ws;

use crate::core::messages::{CoreCommand, CoreEvent};
use std::collections::HashMap;
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

    // One live WebSocket task per device id.
    let mut ws_tasks: HashMap<String, WsTask> = HashMap::new();

    while let Ok(command) = cmd_rx.recv() {
        match command {
            CoreCommand::ExecuteApi(request) => {
                let event = runtime.block_on(http::execute_api_request(request));
                let _ = event_tx.send(event);
            }
            CoreCommand::ConnectWebSocket { device_id, url } => {
                // Replace any existing connection for this device.
                if let Some(task) = ws_tasks.remove(&device_id) {
                    stop_ws_task(&runtime, task);
                }

                let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
                let event_tx_clone = event_tx.clone();
                let loop_device_id = device_id.clone();
                let handle = runtime.spawn(async move {
                    ws::run_ws_loop(loop_device_id, url, stop_rx, event_tx_clone).await;
                });
                ws_tasks.insert(device_id, WsTask { stop_tx, handle });
            }
            CoreCommand::DisconnectWebSocket { device_id } => {
                if let Some(task) = ws_tasks.remove(&device_id) {
                    stop_ws_task(&runtime, task);
                }
                let _ = event_tx.send(CoreEvent::WebSocketDisconnected {
                    device_id,
                    reason: "Disconnected".to_owned(),
                });
            }
            CoreCommand::Shutdown => {
                for (_, task) in ws_tasks.drain() {
                    stop_ws_task(&runtime, task);
                }
                break;
            }
        }
    }
}

/// A running WebSocket task and its stop signal.
struct WsTask {
    stop_tx: tokio::sync::oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

fn stop_ws_task(runtime: &tokio::runtime::Runtime, task: WsTask) {
    let _ = task.stop_tx.send(());
    let _ = runtime.block_on(task.handle);
}
