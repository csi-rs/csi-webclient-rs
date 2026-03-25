use crate::core::messages::CoreEvent;
use futures_util::StreamExt;
use std::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;

/// Run the WebSocket receive loop until remote close, error, or stop signal.
///
/// Every received text/binary payload is forwarded as `CoreEvent::WebSocketFrame`.
/// The caller is responsible for deciding how to interpret payload bytes.
pub async fn run_ws_loop(
    url: String,
    mut stop_rx: oneshot::Receiver<()>,
    event_tx: Sender<CoreEvent>,
) {
    let connect_result = tokio_tungstenite::connect_async(url).await;

    let (mut socket, _) = match connect_result {
        Ok(parts) => parts,
        Err(err) => {
            let _ = event_tx.send(CoreEvent::WebSocketDisconnected {
                reason: format!("WebSocket connect failed: {err}"),
            });
            return;
        }
    };

    let _ = event_tx.send(CoreEvent::WebSocketConnected);

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                let _ = socket.close(None).await;
                let _ = event_tx.send(CoreEvent::WebSocketDisconnected {
                    reason: "Disconnected by user".to_owned(),
                });
                break;
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Binary(bytes))) => {
                        let _ = event_tx.send(CoreEvent::WebSocketFrame(bytes.to_vec()));
                    }
                    Some(Ok(Message::Text(text))) => {
                        let _ = event_tx.send(CoreEvent::WebSocketFrame(text.as_str().as_bytes().to_vec()));
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let reason = frame
                            .map(|f| f.reason.to_string())
                            .filter(|r| !r.is_empty())
                            .unwrap_or_else(|| "Remote closed connection".to_owned());
                        let _ = event_tx.send(CoreEvent::WebSocketDisconnected { reason });
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        let _ = event_tx.send(CoreEvent::WebSocketDisconnected {
                            reason: format!("WebSocket error: {err}"),
                        });
                        break;
                    }
                    None => {
                        let _ = event_tx.send(CoreEvent::WebSocketDisconnected {
                            reason: "WebSocket stream ended".to_owned(),
                        });
                        break;
                    }
                }
            }
        }
    }
}
