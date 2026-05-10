# csi-webclient

Native Rust desktop client for controlling `csi-webserver` through REST and WebSocket APIs.

This crate ships the full application and exposes modules that keep UI code, state, and side
effects separate so behavior stays maintainable as protocol features evolve.

## Module Layout

- `app`: top-level orchestration that translates `UserIntent` values into core commands and
  applies core events back into app state.
- `state`: source-of-truth state models, user intents, and API-facing enum mappings.
- `core`: side-effect layer for HTTP requests, WebSocket receive loop, and worker runtime.
- `ui`: rendering modules for each tab (dashboard/config/control/stream) without direct IO.

## Runtime Flow

1. UI interactions enqueue intents in `state::AppState`.
2. `app::CsiClientApp` drains intents and submits `core::messages::CoreCommand` values.
3. `core` executes HTTP/WebSocket side effects on a worker thread and Tokio runtime.
4. `core` emits `core::messages::CoreEvent` values over channels.
5. `app::CsiClientApp` polls and applies events to state on each frame.

This design avoids blocking work in the egui frame callback and keeps network concerns out of
view code.

## API Coverage

The client targets every documented `csi-webserver` route:

- `GET /api/info`
- `GET /api/config`
- `GET /api/control/status`
- `POST /api/config/reset`
- `POST /api/config/wifi`
- `POST /api/config/traffic`
- `POST /api/config/csi`
- `POST /api/config/collection-mode`
- `POST /api/config/log-mode`
- `POST /api/config/output-mode`
- `POST /api/config/rate`
- `POST /api/config/io-tasks`
- `POST /api/config/csi-delivery`
- `POST /api/control/start`
- `POST /api/control/stop`
- `POST /api/control/reset`
- `POST /api/control/stats`
- `GET /api/ws`

For detailed request/response behavior and payload fields, see:

- <https://github.com/csi-rs/csi-webclient-rs/blob/main/docs/HTTP_API.md>

## Protocol Values Used By The Client

- Wi-Fi modes: `station`, `sniffer`, `esp-now-central`, `esp-now-peripheral`
- Collection modes: `collector`, `listener`
- Log modes: `text`, `array-list`, `serialized`, `esp-csi-tool`
- Output modes: `stream`, `dump`, `both`
- CSI delivery modes: `off`, `callback`, `async`
- PHY rates: `1m`, `1m-l`, `2m`, `5m5`, `5m5-l`, `11m`, `11m-l`, `6m`, `9m`, `12m`,
  `18m`, `24m`, `36m`, `48m`, `54m`, `mcs0-lgi`..`mcs7-lgi`, `mcs0-sgi`

## Validation

Free-form strings sent to `POST /api/config/wifi` (`sta_ssid`,
`sta_password`) are validated client-side to match the firmware tokenizer
rules: max 32 bytes, no newlines, and not both `'` and `"` in the same value.

## Notes

- HTTP success is treated as status code in the `2xx` range.
- Status codes 412 / 503 / 502 / 504 / 403 are mapped to operator-friendly hints
  that explain whether the firmware gate is closed, the ESP32 is disconnected,
  or the WebSocket is blocked by the active output mode.
- API responses are parsed best-effort from either a generic envelope
  (`success`/`message`/`data`) or direct JSON payload.
- WebSocket text and binary messages are both stored as frame bytes for stream inspection.
