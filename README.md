# csi-webclient

Desktop client for configuring and controlling `csi-webserver` remotely.

This project provides a native Rust GUI (egui/eframe) that talks to a running `csi-webserver` instance over HTTP and WebSocket. It is designed for responsive operation, strict architectural separation, and easy troubleshooting during CSI collection sessions.

## Features

- Discover and manage **multiple ESP32 devices** via `GET /api/devices` with automatic hotplug polling (~2 s).
- Per-device configuration, control, and WebSocket streaming under `/api/devices/{id}/...`.
- Fleet-wide **Start All / Stop All** and multi-select synchronized collection (FDM mesh).
- Connect per-device WebSockets and view incoming **serialized** CSI frame previews (COBS+postcard hex).
- Record local **Parquet** exports (`csi_export_{id}_YYYYMMDD_HHmmss.parquet`) with a schema matching server-side dumps.
- Switch runtime output behavior (`stream`, `dump`, `both`) per device from the UI.
- Configure **esp-csi-cli-rs v0.7.0** Wi-Fi modes (`wifi-ap`, ESP-NOW fast simplex) and softAP options.
- Apply two-device **pairing presets** (SoftAP lab, ESP-NOW fast/balanced) from the Devices tab.
- **Save/load device configuration** as JSON snapshots (`csi_config_{id}_YYYYMMDD_HHmmss.json`; note: includes Wi-Fi passwords in plain text) and **copy configuration from one device to another** from the Config tab.

## Architecture

The codebase intentionally separates responsibilities into three domains:

- `src/state`: source of truth for app data and UI-visible state.
- `src/ui`: rendering-only modules (dumb UI, no network/business orchestration).
- `src/core`: side effects (HTTP requests, WebSocket loop, async runtime, channels).
- `src/export`: host-side serialized CSI decoder and Parquet writer.

Top-level intent orchestration and event application happen in `src/app.rs`.

## Documentation

- Crate-level docs for docs.rs are maintained in `docs/CRATE_DOCS.md` (independent from this README).
- HTTP/WebSocket API reference is maintained in `docs/HTTP_API.md`.

## Webserver Compatibility

The client targets **`csi-webserver` ≥ 0.1.5** (multi-device API + esp-csi-cli v0.7.0 Wi-Fi modes). Key endpoints:

- `GET /api/devices` — discover attached boards and live status
- `GET /api/devices/{id}/info`
- `GET /api/devices/{id}/config`
- `GET /api/devices/{id}/control/status`
- `POST /api/devices/{id}/config/*` — wifi, traffic, csi, collection-mode, output-mode, rate, io-tasks, csi-delivery, protocol, reset
- `POST /api/devices/{id}/control/*` — start, stop, reset, stats
- `GET /api/devices/{id}/ws` — per-device WebSocket (raw serialized CSI frames)

The server always runs devices in **serialized** mode; there is no `log-mode` configuration in v0.1.4.

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run --release
```

When the app starts, set host/port in the top bar to match your webserver (default `127.0.0.1:3000`), then click **Connect**. The client polls for attached devices automatically.

Tabs:

- **Devices**: fleet overview, per-device start/stop, refresh, and event log.
- **Dashboard**: per-device status, firmware info, and stream counters.
- **Config**: send per-device configuration endpoints.
- **Control**: start/stop collection, connect/disconnect WebSocket.
- **Stream**: inspect frame counters, hex previews, and record local Parquet exports.

Select one or more devices from the Devices tab or the top-bar combo box to drive the detail tabs side by side.

## Development

```bash
cargo check
cargo test
```

## License

See `LICENSE`.
