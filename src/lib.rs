//! # csi-webserver-cli-rs
//!
//! Native Rust desktop client for controlling `csi-webserver` through REST and WebSocket APIs.
//!
//! This crate contains the full client implementation and is organized around strict domain
//! separation to keep immediate-mode GUI code maintainable:
//!
//! - [`state`] - source-of-truth app state and user intents.
//! - [`ui`] - dumb rendering modules with no direct networking.
//! - [`core`] - side-effect layer (HTTP, WebSocket, async runtime).
//! - [`app`] - orchestration between user intents and core events.
//!
//! ## Design Goals
//!
//! - Keep the frame loop responsive (no blocking work inside `update`).
//! - Keep feature logic isolated by module.
//! - Keep protocol-level integration aligned with `csi-webserver` API semantics.
//! - Make troubleshooting straightforward via runtime status and event logs.
//!
//! ## Runtime Flow
//!
//! 1. UI widgets mutate [`state::AppState`] and queue [`state::UserIntent`] values.
//! 2. [`app::CsiClientApp`] drains intents each frame and submits [`core::messages::CoreCommand`].
//! 3. [`core`] executes side effects on a worker thread/Tokio runtime.
//! 4. [`core`] sends [`core::messages::CoreEvent`] back through channels.
//! 5. [`app::CsiClientApp`] applies events to [`state::AppState`] using non-blocking polling.
//!
//! ## API Coverage
//!
//! The client is designed for `csi-webserver` endpoints:
//!
//! - `GET /api/config`
//! - `POST /api/config/reset`
//! - `POST /api/config/wifi`
//! - `POST /api/config/traffic`
//! - `POST /api/config/csi`
//! - `POST /api/config/collection-mode`
//! - `POST /api/config/log-mode`
//! - `POST /api/config/output-mode`
//! - `POST /api/control/start`
//! - `POST /api/control/reset`
//! - `GET /api/ws`
//!
//! ## Log Modes
//!
//! The log mode model follows the reworked server contract:
//!
//! - `text`
//! - `array-list`
//! - `serialized`
//!
//! ## Usage
//!
//! This crate is primarily consumed as a desktop binary (`src/main.rs`).
//! Library docs are provided to make module contracts and state/core APIs explicit.

pub mod app;
pub mod core;
pub mod state;
pub mod ui;
