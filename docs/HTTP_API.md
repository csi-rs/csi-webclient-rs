# HTTP and WebSocket API Reference

This reference documents the endpoints and payloads that `csi-webclient`
issues against `csi-webserver` **0.3.0+** (with `esp-csi-cli-rs` 0.8.0+ on the
devices). Behavior, gating rules, and
validation match the server-side specification.

## Base Addresses

- HTTP base URL: `http://{host}:{port}`
- WebSocket URL (per device): `ws://{host}:{port}/api/devices/{id}/ws`

Default host/port in the app: `127.0.0.1:3000`.

All per-device routes require a device `id` discovered via `GET /api/devices`.
An unknown id returns `404 Not Found`.

## Device Discovery

### `GET /api/devices`

Lists every attached ESP32 and its runtime status. Always reachable (no
firmware gate). The client polls this every ~2 s for hotplug discovery.

```json
[
  {
    "id": "ttyUSB0",
    "port_path": "/dev/ttyUSB0",
    "baud_rate": 115200,
    "serial_connected": true,
    "collection_running": false,
    "firmware_verified": true,
    "device_info": {
      "banner_version": "0.5.0",
      "name": "esp-csi-cli-rs",
      "version": "0.5.0",
      "chip": "esp32c6",
      "mac": "D0:CF:13:E2:90:E8",
      "protocol": 2,
      "features": ["statistics"]
    },
    "fault": null
  }
]
```

- `device_info` is `null` until the device's firmware is verified.
- Default `id` is the sanitized port basename; MAC-based ids are used when available.
- `fault` (optional string) is set when the server detects a known chip fault
  from the boot output — e.g. the ESP32-C5/C6 USB-JTAG reset-loop wedge
  (`rst:0x15 USB_UART_HPSYS`, only recoverable by a USB power cycle), ROM
  download mode, or a generic boot loop. The client shows a red FAULT badge
  in the Firmware column and a per-device banner with the recovery action;
  it clears automatically once the device verifies again.

## Identity and Status (per device)

### `GET /api/devices/{id}/info`

Verifies firmware identity. The response shape is:

```json
{
  "banner_version": "0.5.0",
  "name": "esp-csi-cli-rs",
  "version": "0.5.0",
  "chip": "esp32c6",
  "protocol": 1,
  "features": ["statistics", "println", "auto"]
}
```

Status codes:

- `200 OK` — firmware confirmed; cache refreshed.
- `404 Not Found` — no device with this id.
- `412 Precondition Failed` — firmware not verified on this device.
- `502 Bad Gateway` — magic block malformed.
- `503 Service Unavailable` — ESP32 disconnected, or a session is running.
- `504 Gateway Timeout` — firmware is most likely not `esp-csi-cli-rs`.

### `GET /api/devices/{id}/control/status`

Returns runtime state for one device. Always reachable; not gated by firmware verification.

```json
{
  "serial_connected": true,
  "collection_running": false,
  "port_path": "/dev/ttyUSB0"
}
```

## Config Endpoints (per device)

All paths below are prefixed with `/api/devices/{id}/`.

### `GET /api/devices/{id}/config`

Returns the server-side cached config, mirroring the firmware's
`show-config` sections. Every field is nullable; absent fields mean the
matching `POST /api/devices/{id}/config/*` endpoint has not been invoked
since startup or the last `reset-config`. Sub-section objects (`wifi`,
`collection`, `csi_config`) are present even when empty.

```json
{
  "wifi": {
    "mode": "sniffer",
    "channel": 6,
    "sta_ssid": "MyNetwork"
  },
  "collection": {
    "csi_output_enabled": true,
    "traffic_hz": 100,
    "phy_rate": "mcs0-lgi",
    "protocol": "n",
    "io_tx_enabled": true,
    "io_rx_enabled": true
  },
  "csi_config": {
    "lltf_enabled": true,
    "htltf_enabled": true,
    "stbc_htltf_enabled": true,
    "ltf_merge_enabled": true,
    "channel_filter_enabled": false,
    "manual_scale": false,
    "shift": 0,
    "dump_ack_enabled": false,
    "acquire_csi": 1,
    "acquire_csi_legacy": 1,
    "acquire_csi_ht20": 1,
    "acquire_csi_ht40": 1,
    "val_scale_cfg": 2
  },
  "csi_delivery_mode": "async",
  "csi_logging_enabled": true
}
```

Notes the client relies on:

- `csi_config` carries both classic-chip booleans (`lltf_enabled`,
  `htltf_enabled`, `stbc_htltf_enabled`, `ltf_merge_enabled`) and ESP32-C5/C6
  `acquire_csi*` integers. Only the side that matches the connected chip
  is populated; the other stays `null`.
- The CSI form mirrors the server's on/off toggles (`lltf`, `csi_legacy`, …).
  `GET …/config` returns `*_enabled` booleans and `acquire_csi*` integers;
  the client maps `acquire_csi* != 0` to enabled.
- `channel_filter_enabled`, `manual_scale`, `shift` are read-only on the device —
  the client surfaces them but cannot set them via `POST /api/devices/{id}/config/csi`.
- `dump_ack`, `csi_force_lltf`, and `csi_vht` are configurable on the ESP32-C5/C6.
- `sta_password` is **not** in the response by design.
- **`log_mode` is removed in v0.1.4** — the server always runs serialized mode.

### `POST /api/devices/{id}/config/reset`

- Body: none.
- Resets device-side configuration and clears the server cache.

### `POST /api/devices/{id}/config/wifi`

```json
{
  "mode": "station | sniffer | wifi-ap | ht20-emitter | ht40-emitter | esp-now-central | esp-now-peripheral | esp-now-fast-source | esp-now-fast-collector",
  "sta_ssid": "string or null",
  "sta_password": "string or null",
  "ap_ssid": "string",
  "ap_password": "string",
  "ap_dhcp": true,
  "ap_leases": 4,
  "ap_burst": false,
  "channel": 6,
  "peer_mac": "string or null",
  "ht40": "none | above | below",
  "collection": "collector | listener"
}
```

Client-side validation (mirrors firmware tokenizer rules):

- `sta_ssid` / `sta_password`: max 32 bytes (UTF-8); sent only in `station` mode.
- `ap_ssid` / `ap_password`: max 32 bytes; sent only in `wifi-ap` mode.
- `ap_dhcp`: boolean; sent only in `wifi-ap` mode.
- `ap_leases`: integer 1–8 (DHCP lease pool size; firmware default 4); sent
  only in `wifi-ap` mode. The UI clamps the value to the valid range.
- `ap_burst`: boolean (synchronized burst flood — every flood tick sends one
  frame to every associated station for time-aligned multi-receiver CSI;
  total airtime = `frequency_hz × leases`); sent only in `wifi-ap` mode.
- Newlines (`\r`, `\n`) are rejected.
- Values containing both `'` and `"` are rejected — the firmware
  tokenizer cannot disambiguate them.
- `channel` is optional and forwarded whenever the field is non-empty. In
  `station` mode it is a pre-association band-selection hint (meaningful on the
  C5's 5 GHz band); leave it blank to inherit the channel from the associated AP.
  For all other modes it is the operating channel.
- `mode` names the node's **operational mode** — how it reaches the channel. The
  node model (network role, collection mode, operational mode, session role) is
  documented in
  [`esp-csi-rs/docs/network-model.md`](https://github.com/csi-rs/esp-csi-rs/blob/main/docs/network-model.md).
  `ht20-emitter` / `ht40-emitter` raw-inject HT PPDUs without associating and
  capture nothing; they build on every chip. The simplex ends are sent as the
  firmware's `esp-now-fast-source` / `esp-now-fast-collector`; the server also
  accepts `esp-now-simplex-source` / `esp-now-simplex-peer`. A [`ClientProfile`]
  may add further modes (see "Unknown modes" below).
- `peer_mac` is sent in the emitter and ESP-NOW modes. In the emitter modes it is
  the destination address of the injected frames, empty meaning broadcast;
  unicasting to a collector's MAC usually raises that collector's CSI rate. In
  the ESP-NOW modes it is the explicit peer, empty meaning automatic pairing;
  when set, configure both nodes, each with the other's address.
- `ht40` is sent in `wifi-ap` and the ESP-NOW modes. In `wifi-ap` it runs the
  softAP itself as HT40 with the given secondary channel (`none` = HT20); in the
  ESP-NOW modes it forces the per-peer TX PHY to HT40. It never selects emitter
  bandwidth — that is the `ht40-emitter` mode.
- `collection` is the node's collection mode, forwarded as
  `set-wifi --collection=<value>`: a `collector` reports the CSI it captures, a
  `listener` captures but does not report. It is optional, and the client sends it
  only for `station`, `wifi-ap`, `esp-now-central` and `esp-now-peripheral`, and
  only when the user picked a value (unset keeps the firmware default,
  collector). The server rejects it with `400` for the modes that fix the
  collection mode (`sniffer`, the emitters, the simplex ends), checked against
  the `mode` in the same request.
- Mode `wifi-ap` requires `esp-csi-cli-rs` ≥ 0.7.0; the client gates it in the
  mode picker.

Unknown modes: a mode string the client does not name round-trips verbatim
(`WiFiMode::Ext`) instead of failing, so the client degrades gracefully against
firmware whose mode vocabulary it predates.

[`ClientProfile`]: ../src/profile.rs

### `POST /api/devices/{id}/config/traffic`

```json
{ "frequency_hz": 1000, "unsolicited": true }
```

`frequency_hz` is required and parses as `u64`. `0` disables traffic.

`unsolicited` is optional (`bool`): `true` makes the ICMP flood send
unsolicited echo replies — one-directional traffic (peer never answers at the
IP level), stable offered rate, but the flooding node captures no CSI back.
The client sends it only for WiFi AP/station modes and omits it otherwise, so
older firmware never sees an unknown flag. Round-trips via the `collection`
section (`unsolicited`) of `GET …/config`.

### `POST /api/devices/{id}/config/csi`

```json
{
  "lltf": true,
  "htltf": true,
  "stbc_htltf": true,
  "ltf_merge": true,
  "csi": true,
  "csi_legacy": true,
  "csi_ht20": true,
  "csi_ht40": true,
  "dump_ack": true,
  "csi_force_lltf": true,
  "csi_vht": true,
  "preset": "default",
  "val_scale_cfg": 2
}
```

All fields are optional. Boolean flags use explicit on/off semantics (v0.1.5+).
`preset` selects a named full CSI-acquisition preset (`default`; companion
builds may add more). `val_scale_cfg` parses as `u32` when present. The flag
groupings are documented in the server spec; the firmware silently ignores
flags outside its compiled-in chip variant.

### `POST /api/devices/{id}/config/csi-output`

```json
{ "enabled": true }
```

Forwards `set-csi-output --enabled=<true|false>` (device default `true`):
whether captured CSI is delivered off-device. This is *not* a role — capture
keeps running either way, so the RX path and its timing are unchanged; with
delivery off nothing is decoded, logged, or streamed. Use it for a node whose
only job is to keep traffic on air. An emitter captures nothing, so the flag has
no effect there. Round-trips via the `collection` section
(`csi_output_enabled`) of `GET …/config`.

### `POST /api/devices/{id}/config/output-mode`

```json
{ "mode": "stream | dump | both" }
```

Server-side fan-out only. While `mode == "dump"`, `GET /api/devices/{id}/ws`
returns `403 Forbidden`.

When `mode` is `dump` or `both`, the server writes session dumps as
**Parquet** files: `csi_dump_{id}_YYYYMMDD_HHmmss.parquet`.

### `POST /api/devices/{id}/config/protocol`

```json
{ "protocol": "b | g | n | lr | a | ac" }
```

Applied at the start of each collection run. Default on the device is `lr`.

### `POST /api/devices/{id}/config/rate`

```json
{ "rate": "mcs0-lgi" }
```

Accepted rates: `1m`, `1m-l`, `2m`, `5m5`, `5m5-l`, `11m`, `11m-l`, `6m`,
`9m`, `12m`, `18m`, `24m`, `36m`, `48m`, `54m`, `mcs0-lgi`..`mcs7-lgi`,
`mcs0-sgi`. Reporting only (recorded in the device config), except on the
ESP-NOW pair (`esp-now-central` / `esp-now-peripheral`), which applies it as the
TX PHY rate.

### `POST /api/devices/{id}/config/io-tasks`

```json
{ "tx": true, "rx": true }
```

Both fields optional; omitted ones preserve the device's current value.

### `POST /api/devices/{id}/config/csi-delivery`

```json
{ "mode": "off | callback | async | raw", "logging": true }
```

Both fields optional, but at least one must be present (the server
returns `400` otherwise). Takes effect immediately on the firmware.

## Control Endpoints (per device)

All paths below are prefixed with `/api/devices/{id}/`.

### `POST /api/devices/{id}/control/start`

```json
{ "duration": 30 }
```

`duration` is optional (`u64` seconds). Empty input → no body, indefinite
collection.

### `POST /api/devices/{id}/control/stop`

- Body: none.
- Sends the literal `q` byte; firmware unwinds gracefully.

### `POST /api/devices/{id}/control/reset`

- Body: none.
- Pulses RTS to hard-reset the ESP32 and re-runs firmware verification.

### `POST /api/devices/{id}/control/stats`

- Body: none.
- Triggers `show-stats`. Counter values appear in the CSI output stream;
  the HTTP response is just an acknowledgment.

## WebSocket Stream (per device)

### `GET /api/devices/{id}/ws`

- Upgraded to WebSocket by the client.
- Carries **raw serialized CSI frames** (COBS-framed postcard records) for
  that device only.
- Binary frames are forwarded as raw bytes.
- Text frames are converted to bytes and handled through the same path.
- When the device is unplugged, the server sends a WebSocket Close frame;
  the client re-discovers via `GET /api/devices`.

## Local Parquet Export (client-side)

The client can independently record WebSocket frames to Parquet files on
the host:

- Filename: `csi_export_{id}_YYYYMMDD_HHmmss.parquet`
- Schema matches server-side `csi_dump_*.parquet` (superset, chip-specific
  columns nullable, includes `host_rx_time`).

## Status-Code Hints Surfaced By The Client

| Status                  | Hint                                                            |
|-------------------------|-----------------------------------------------------------------|
| `404 Not Found`         | Device not found — it may have been unplugged.                 |
| `412 Precondition Failed` | Firmware not verified — try Fetch Info or Reset Device.       |
| `503 Service Unavailable` | ESP32 not connected, or operation invalid for current state.  |
| `502 Bad Gateway`         | Device responded but the info block was malformed.            |
| `504 Gateway Timeout`     | Info block timed out — firmware may not be `esp-csi-cli-rs`.  |
| `403 Forbidden` (WebSocket) | Output mode is `dump` — switch to `stream`/`both` first.    |

## Response Handling in Client

- HTTP status `2xx` is success.
- Empty body: success → "Request completed"; failure → "Request failed".
- Non-empty body is parsed best-effort as JSON; envelope `message` and
  `data` fields are preferred when present.
- A `404` on any per-device route triggers a device-list refresh.
