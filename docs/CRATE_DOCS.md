# csi-webclient

Native Rust desktop client for controlling `csi-webserver` through REST and WebSocket APIs.

This crate ships the full application and exposes modules that keep UI code, state, and side
effects separate so behavior stays maintainable as protocol features evolve.

## Module Layout

- `app`: top-level orchestration that translates `UserIntent` values into core commands and
  applies core events back into app state.
- `state`: source-of-truth state models, user intents, and API-facing enum mappings.
- `core`: side-effect layer for HTTP requests, WebSocket receive loop, and worker runtime.
- `export`: host-side serialized CSI decoder and Parquet writer for local recordings.
- `ui`: rendering modules for each tab (devices/dashboard/config/control/stream) without direct IO.

## Runtime Flow

1. UI interactions enqueue intents in `state::AppState`.
2. `app::CsiClientApp` drains intents and submits `core::messages::CoreCommand` values.
3. `core` executes HTTP/WebSocket side effects on a worker thread and Tokio runtime.
4. `core` emits `core::messages::CoreEvent` values over channels.
5. `app::CsiClientApp` polls and applies events to state on each frame.

This design avoids blocking work in the egui frame callback and keeps network concerns out of
view code.

## API Coverage

The client targets `csi-webserver` **v0.1.5+** (multi-device API + esp-csi-cli v0.7.0 Wi-Fi modes):

- `GET /api/devices`
- `GET /api/devices/{id}/info`
- `GET /api/devices/{id}/config`
- `GET /api/devices/{id}/control/status`
- `POST /api/devices/{id}/config/reset`
- `POST /api/devices/{id}/config/wifi`
- `POST /api/devices/{id}/config/traffic`
- `POST /api/devices/{id}/config/csi`
- `POST /api/devices/{id}/config/csi-output`
- `POST /api/devices/{id}/config/output-mode`
- `POST /api/devices/{id}/config/protocol`
- `POST /api/devices/{id}/config/rate`
- `POST /api/devices/{id}/config/io-tasks`
- `POST /api/devices/{id}/config/csi-delivery`
- `POST /api/devices/{id}/control/start`
- `POST /api/devices/{id}/control/stop`
- `POST /api/devices/{id}/control/reset`
- `POST /api/devices/{id}/control/stats`
- `GET /api/devices/{id}/ws`

For detailed request/response behavior and payload fields, see:

- <https://github.com/csi-rs/csi-webclient-rs/blob/main/docs/HTTP_API.md>

## Protocol Values Used By The Client

- Wi-Fi modes: `station`, `sniffer`, `wifi-ap` (requires firmware ≥ 0.7.0) — the
  collector capture paths — plus the TX-only `ht20-emitter`, `ht40-emitter`
  (all chips). A `ClientProfile` may add more; any mode string the client does
  not name round-trips verbatim as `WiFiMode::Ext`.
- CSI output (`config/csi-output`): `enabled` boolean, device default `true` —
  whether captured CSI is delivered off-device. Capture runs either way.
- Output modes: `stream`, `dump`, `both`
- CSI delivery modes: `off`, `callback`, `async`, `raw`
- Wi-Fi PHY protocols: `b`, `g`, `n`, `lr`, `a`, `ac` (a `ClientProfile` may add more)
- Two-device pairing presets: SoftAP lab, HT20 emitter + sniffer, HT40 emitter + sniffer
- PHY rates: `1m`, `1m-l`, `2m`, `5m5`, `5m5-l`, `11m`, `11m-l`, `6m`, `9m`, `12m`,
  `18m`, `24m`, `36m`, `48m`, `54m`, `mcs0-lgi`..`mcs7-lgi`, `mcs0-sgi`

The server always delivers CSI in **serialized** (COBS+postcard) mode;
there is no `log-mode` configuration.

## Validation

Free-form strings sent to `POST /api/devices/{id}/config/wifi` (`sta_ssid`,
`sta_password`, `ap_ssid`, `ap_password`) are validated client-side to match
the firmware tokenizer rules: max 32 bytes, no newlines, and not both `'`
and `"` in the same value. Mode-specific fields are omitted from the JSON
body when they do not apply (STA fields in station mode only, AP fields in
`wifi-ap` only, peer MAC in the emitter modes only, HT40 in `wifi-ap` only).

## Notes

- HTTP success is treated as status code in the `2xx` range.
- Status codes 404 / 412 / 503 / 502 / 504 / 403 are mapped to operator-friendly hints
  that explain whether the device is gone, the firmware gate is closed, the ESP32 is
  disconnected, or the WebSocket is blocked by the active output mode.
- API responses are parsed best-effort from either a generic envelope
  (`success`/`message`/`data`) or direct JSON payload.
- WebSocket text and binary messages are both stored as frame bytes for stream inspection.
- Local Parquet recordings use the same schema as server-side `csi_dump_*.parquet` files.
