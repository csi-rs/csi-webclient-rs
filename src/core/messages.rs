use serde_json::Value;

/// Commands sent from the app orchestration layer to the core worker.
#[derive(Debug, Clone)]
pub enum CoreCommand {
    /// Execute one HTTP request against the configured webserver.
    ExecuteApi(ApiRequest),
    /// Open or replace the active WebSocket stream connection.
    ConnectWebSocket { url: String },
    /// Stop the active WebSocket stream connection, if any.
    DisconnectWebSocket,
    /// Stop the core worker gracefully.
    Shutdown,
}

/// A normalized HTTP request model consumed by the core worker.
#[derive(Debug, Clone)]
pub struct ApiRequest {
    /// Logical operation label used for UI messages and event correlation.
    pub label: String,
    /// HTTP verb to send.
    pub method: HttpMethod,
    /// Base URL (for example, `http://127.0.0.1:3000`).
    pub base_url: String,
    /// Request path (for example, `/api/config/wifi`).
    pub path: String,
    /// Optional JSON body.
    pub body: Option<Value>,
}

/// Event payload returned to the app after an HTTP request.
#[derive(Debug, Clone)]
pub struct ApiResponseEvent {
    /// Logical request label echoed from [`ApiRequest::label`].
    pub label: String,
    /// True when status code is in the 2xx range.
    pub success: bool,
    /// HTTP status code. `0` is used for transport-level failures.
    pub status: u16,
    /// Human-readable message for status/error UI.
    pub message: String,
    /// Parsed response payload, if available.
    pub data: Option<Value>,
}

/// Events emitted by the core worker and consumed by the app layer.
#[derive(Debug, Clone)]
pub enum CoreEvent {
    /// HTTP request completed.
    ApiResponse(ApiResponseEvent),
    /// WebSocket connection successfully established.
    WebSocketConnected,
    /// WebSocket connection ended.
    WebSocketDisconnected { reason: String },
    /// One WebSocket payload received from server.
    WebSocketFrame(Vec<u8>),
    /// Internal diagnostic log line from core worker/runtime.
    Log(String),
}

/// Supported HTTP methods in this client.
#[derive(Debug, Clone)]
pub enum HttpMethod {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
}
