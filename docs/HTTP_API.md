# HTTP and WebSocket API Reference

This reference documents the endpoints and payloads that `csi-webclient`
issues against `csi-webserver`. Behavior, gating rules, and validation
match the server-side specification.

## Base Addresses

- HTTP base URL: `http://{host}:{port}`
- WebSocket URL: `ws://{host}:{port}/api/ws`

Default host/port in the app: `127.0.0.1:3000`.

## Identity and Status

### `GET /api/info`

Verifies firmware identity. The response shape is:

```json
{
  "banner_version": "0.4.0",
  "name": "esp-csi-cli-rs",
  "version": "0.4.0",
  "chip": "esp32c6",
  "protocol": 1,
  "features": ["statistics", "println", "auto"]
}
```

Status codes:

- `200 OK` — firmware confirmed; cache refreshed.
- `502 Bad Gateway` — magic block malformed.
- `503 Service Unavailable` — ESP32 disconnected, or a session is running.
- `504 Gateway Timeout` — firmware is most likely not `esp-csi-cli-rs`.

### `GET /api/control/status`

Returns runtime state. Always reachable; not gated by firmware verification.

```json
{
  "serial_connected": true,
  "collection_running": false,
  "port_path": "/dev/ttyUSB0"
}
```

## Config Endpoints

### `GET /api/config`

Returns the server-side cached config, mirroring the firmware's
`show-config` sections. Every field is nullable; absent fields mean the
matching `POST /api/config/*` endpoint has not been invoked since startup
or the last `reset-config`. Sub-section objects (`wifi`, `collection`,
`csi_config`) are present even when empty.

```json
{
  "wifi": {
    "mode": "sniffer",
    "channel": 6,
    "sta_ssid": "MyNetwork"
  },
  "collection": {
    "mode": "collector",
    "traffic_hz": 100,
    "phy_rate": "mcs0-lgi",
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
    "acquire_csi_su": 1,
    "acquire_csi_mu": 1,
    "acquire_csi_dcm": 1,
    "acquire_csi_beamformed": 1,
    "csi_he_stbc": 2,
    "val_scale_cfg": 2
  },
  "log_mode": "array-list",
  "csi_delivery_mode": "async",
  "csi_logging_enabled": true
}
```

Notes the client relies on:

- `csi_config` carries both classic-chip booleans (`lltf_enabled`,
  `htltf_enabled`, `stbc_htltf_enabled`, `ltf_merge_enabled`) and HE-chip
  `acquire_csi*` integers. Only the side that matches the connected chip
  is populated; the other stays `null`.
- The CSI form sends `disable_*` flags, so applying the cache inverts each
  `*_enabled` boolean and treats `acquire_csi* == 0` as disabled.
- `channel_filter_enabled`, `manual_scale`, `shift`, `dump_ack_enabled`
  are read-only on the device — the client surfaces them but cannot set
  them via `POST /api/config/csi`.
- `sta_password` is **not** in the response by design.

### `POST /api/config/reset`

- Body: none.
- Resets device-side configuration and clears the server cache.

### `POST /api/config/wifi`

```json
{
  "mode": "station | sniffer | esp-now-central | esp-now-peripheral",
  "sta_ssid": "string or null",
  "sta_password": "string or null",
  "channel": 6
}
```

Client-side validation (mirrors firmware tokenizer rules):

- `sta_ssid` / `sta_password`: max 32 bytes (UTF-8); empty input becomes `null`.
- Newlines (`\r`, `\n`) are rejected.
- Values containing both `'` and `"` are rejected — the firmware
  tokenizer cannot disambiguate them.
- `channel` is optional; ignored by `station` (which inherits the AP's channel).

### `POST /api/config/traffic`

```json
{ "frequency_hz": 100 }
```

`frequency_hz` is required and parses as `u64`. `0` disables traffic.

### `POST /api/config/csi`

```json
{
  "disable_lltf": false,
  "disable_htltf": false,
  "disable_stbc_htltf": false,
  "disable_ltf_merge": false,
  "disable_csi": false,
  "disable_csi_legacy": false,
  "disable_csi_ht20": false,
  "disable_csi_ht40": false,
  "disable_csi_su": false,
  "disable_csi_mu": false,
  "disable_csi_dcm": false,
  "disable_csi_beamformed": false,
  "csi_he_stbc": 2,
  "val_scale_cfg": 2
}
```

`csi_he_stbc` and `val_scale_cfg` are required and parse as `u32`. The
classic-vs-HE flag groupings are documented in the server spec; the
firmware silently ignores flags outside its compiled-in chip variant.

### `POST /api/config/collection-mode`

```json
{ "mode": "collector | listener" }
```

### `POST /api/config/log-mode`

```json
{ "mode": "text | array-list | serialized | esp-csi-tool" }
```

Takes effect immediately on the firmware *and* the server-side framer.

### `POST /api/config/output-mode`

```json
{ "mode": "stream | dump | both" }
```

Server-side fan-out only. While `mode == "dump"`, `GET /api/ws` returns
`403 Forbidden`.

### `POST /api/config/rate`

```json
{ "rate": "mcs0-lgi" }
```

Accepted rates: `1m`, `1m-l`, `2m`, `5m5`, `5m5-l`, `11m`, `11m-l`, `6m`,
`9m`, `12m`, `18m`, `24m`, `36m`, `48m`, `54m`, `mcs0-lgi`..`mcs7-lgi`,
`mcs0-sgi`. Honored only by ESP-NOW central/peripheral modes.

### `POST /api/config/io-tasks`

```json
{ "tx": true, "rx": true }
```

Both fields optional; omitted ones preserve the device's current value.

### `POST /api/config/csi-delivery`

```json
{ "mode": "off | callback | async", "logging": true }
```

Both fields optional, but at least one must be present (the server
returns `400` otherwise). Takes effect immediately on the firmware.

## Control Endpoints

### `POST /api/control/start`

```json
{ "duration": 30 }
```

`duration` is optional (`u64` seconds). Empty input → no body, indefinite
collection.

### `POST /api/control/stop`

- Body: none.
- Sends the literal `q` byte; firmware unwinds gracefully.

### `POST /api/control/reset`

- Body: none.
- Pulses RTS to hard-reset the ESP32 and re-runs firmware verification.

### `POST /api/control/stats`

- Body: none.
- Triggers `show-stats`. Counter values appear in the CSI output stream;
  the HTTP response is just an acknowledgment.

## WebSocket Stream

### `GET /api/ws`

- Upgraded to WebSocket by the client.
- Binary frames are forwarded as raw bytes.
- Text frames are converted to bytes and handled through the same path.

## Status-Code Hints Surfaced By The Client

| Status                  | Hint                                                            |
|-------------------------|-----------------------------------------------------------------|
| `412 Precondition Failed` | Firmware not verified — try Fetch Info or Reset Device.       |
| `503 Service Unavailable` | ESP32 not connected, or operation invalid for current state.  |
| `502 Bad Gateway`         | Device responded but the info block was malformed.            |
| `504 Gateway Timeout`     | Info block timed out — firmware may not be `esp-csi-cli-rs`.  |
| `403 Forbidden` (`/api/ws`) | Output mode is `dump` — switch to `stream`/`both` first.    |

## Response Handling in Client

- HTTP status `2xx` is success.
- Empty body: success → "Request completed"; failure → "Request failed".
- Non-empty body is parsed best-effort as JSON; envelope `message` and
  `data` fields are preferred when present.
