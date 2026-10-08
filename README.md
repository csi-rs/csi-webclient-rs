# csi-webclient

Desktop client for configuring and controlling `csi-webserver` remotely.

This project provides a native Rust GUI (egui/eframe) that talks to a running `csi-webserver` instance over HTTP and WebSocket. It is designed for responsive operation, strict architectural separation, and easy troubleshooting during CSI collection sessions.

## Features

- Discover and manage **multiple ESP32 devices** via `GET /api/devices` with automatic hotplug polling (~2 s).
- Per-device configuration, control, and WebSocket streaming under `/api/devices/{id}/...`.
- Fleet-wide **Start All / Stop All** and multi-select synchronized collection across several nodes (for example a star of
  peripherals around one central, or a point-to-point pair).
- Connect per-device WebSockets and view incoming **serialized** CSI frame previews (COBS+postcard hex).
- Record local **Parquet** exports (`csi_export_{id}_YYYYMMDD_HHmmss.parquet`) with a versioned schema
  (`schema_version` file metadata).
- Switch runtime output behavior (`stream`, `dump`, `both`) per device from the UI.
- Configure the node's **operational mode** — how it reaches the channel: `station`, `sniffer`,
  `wifi-ap`, `ht20-emitter`, `ht40-emitter`, and the connectionless `esp-now-*` modes. The node
  model is documented once, in
  [`esp-csi-rs/docs/network-model.md`](https://github.com/csi-rs/esp-csi-rs/blob/main/docs/network-model.md).
- Toggle off-device **CSI output** per device (capture keeps running when delivery is off).
- Apply two-device **pairing presets** from the Devices tab: SoftAP lab, HT20/HT40 emitter +
  sniffer, ESP-NOW pair, and ESP-NOW simplex pair.
- **Save/load device configuration** as JSON snapshots (`csi_config_{id}_YYYYMMDD_HHmmss.json`; note: includes Wi-Fi passwords in plain text) and **copy configuration from one device to another** from the Config tab.

## Architecture

The codebase intentionally separates responsibilities into three domains:

- `src/state`: source of truth for app data and UI-visible state.
- `src/ui`: rendering-only modules (dumb UI, no network/business orchestration).
- `src/core`: side effects (HTTP requests, WebSocket loop, async runtime, channels).
- `src/export`: host-side serialized CSI decoder and Parquet writer.
- `src/wire`: the esp-csi-rs wire contract, vendored (see below).

Top-level intent orchestration and event application happen in `src/app.rs`.

## Documentation

- Crate-level docs for docs.rs are maintained in `docs/CRATE_DOCS.md` (independent from this README).
- HTTP/WebSocket API reference is maintained in `docs/HTTP_API.md`.

## Webserver Compatibility

The client targets **`csi-webserver` ≥ 0.3.0**, with `esp-csi-cli-rs` ≥ 0.8.0 on the devices. Key endpoints:

- `GET /api/devices` — discover attached boards and live status
- `GET /api/devices/{id}/info`
- `GET /api/devices/{id}/config`
- `GET /api/devices/{id}/control/status`
- `POST /api/devices/{id}/config/*` — wifi, traffic, csi, csi-output, output-mode, rate, io-tasks, csi-delivery, protocol, reset
- `POST /api/devices/{id}/control/*` — start, stop, reset, stats
- `GET /api/devices/{id}/ws` — per-device WebSocket (raw serialized CSI frames)

The server always runs devices in **serialized** mode, so there is no `log-mode` configuration on
this surface.

## Firmware Wire Format

Recordings decode the versioned wire format of **esp-csi-rs 0.12** (`WIRE_VERSION` 1). Firmware from
0.8 to 0.11 is still decoded, from the device's chip. A frame from a newer wire version is counted as
a decode error, not mis-read.

`src/wire` is a copy of `esp-csi-rs/src/lib/wire` at the 0.12.0 tag. Do not edit it by hand: copy
the files again when the firmware's wire format changes.

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
