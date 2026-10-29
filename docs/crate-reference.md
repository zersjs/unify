# Crate and configuration reference

Unify is organized as a Cargo workspace with seven Rust crates under `crates/` plus a Tauri 2 desktop shell under `app/`. Platform-specific OS bindings exist only in `unify-platform`; every other crate is cross-platform Rust.

## Dependency graph

```text
unify-proto
    ↑
unify-core
    ↑
    ├── unify-security
    ├── unify-net
    └── unify-platform
            ↑
      unify-runtime
        ↑       ↑
  unify-cli   app/src-tauri
```

## Workspace crates

### `crates/unify-proto`

Wire-level types, binary framing, bounds validation, and keyboard scan-code tables. Contains zero network I/O or OS calls.

- **Modules:** `frame`, `keymap`, `message`.
- **Framing:** 4-byte big-endian `u32` payload length followed by `postcard`-encoded `Message`.
- **Key protocol constants:**
  - `PROTOCOL_VERSION = 1`
  - `FRAME_HEADER_LEN = 4` bytes
  - `MAX_FRAME_LEN = 65_536` bytes (`64 KiB`)
  - `CLIPBOARD_CHUNK_LEN = 16_384` bytes (`16 KiB`)
  - `MAX_CLIPBOARD_TEXT_LEN = 16_777_216` bytes (`16 MiB`)
  - `MAX_DEVICE_NAME_CHARS = 64`
  - `MAX_DEVICE_ID_LEN = 64` ASCII hex characters
  - `RATIO_SCALE = 10_000` (maps `0.0..=1.0` screen edge position to `0..=10_000`)
  - `SPAKE_MESSAGE_LEN = 33` bytes
- **Validation (`Message::validate`):** rejects oversized device names, non-hex device IDs, edge ratios above `10_000`, mouse/wheel deltas exceeding `2^20`, clipboard transfers above `16 MiB`, empty or `> 16 KiB` clipboard chunks, and malformed SPAKE2 payloads.

### `crates/unify-core`

Deterministic state machines and configuration persistence, tested independently of sockets and native hooks.

- **Modules:** `backoff`, `capture`, `clipboard`, `config`, `geometry`, `keys`, `switcher`.
- **Timing constants:**
  - `PING_INTERVAL_MS = 1_000` (`1 s` heartbeat)
  - `PEER_TIMEOUT_MS = 5_000` (`5 s` silence closes session)
  - `SLEEP_GAP_MS = 6_000` (`> 6 s` timer jump triggers immediate reconnect after OS wake)
  - `CONNECT_TIMEOUT_MS = 3_000`
  - `HANDSHAKE_TIMEOUT_MS = 5_000`
- **`switcher`:** edge-switch state machine enforcing corner guard zones, optional edge dwell (`0..=1000 ms`), 250 ms return cooldown, and edge re-arming so the pointer cannot bounce across the boundary.
- **`keys`:** tracks locally and remotely held USB HID usages, translates `Command ↔ Control` when enabled, filters Windows synthetic left-Ctrl on `AltGr`, and generates key-up events when control ends or disconnects.
- **`clipboard`:** chunks outgoing UTF-8 strings into `16 KiB` pieces, reassembles incoming chunks, verifies SHA-256 digests, and suppresses echo loops by tracking last sent and last received hashes.

### `crates/unify-security`

Cryptographic identity, OS secret storage, mutual TLS 1.3 configuration, and SPAKE2 pairing.

- **Modules:** `identity`, `keystore`, `pairing`, `tls`.
- **Device identity:** generates ECDSA P-256 key pairs (`rcgen`) and self-signed X.509 certificates; derives SHA-256 `Fingerprint`.
- **Secret storage (`SecretStore`):** `OsSecretStore` persists private keys in macOS Keychain or Windows Credential Manager (`keyring`); `MemorySecretStore` backs deterministic tests.
- **TLS 1.3 (`rustls` + `ring`):** TLS 1.3 only, 0-RTT disabled, session tickets disabled, mutual client/server certificate verification pinned to the paired SHA-256 fingerprint.
- **Pairing constants:**
  - `PAIRING_CODE_DIGITS = 6`
  - `PAIRING_CODE_TTL_MS = 120_000` (`120 s`)
  - `MAX_PAIRING_FAILURES = 5`
  - `PAIRING_CONFIRM_TIMEOUT_MS = 60_000` (`60 s` user approval timeout)
  - Confirmation HMAC-SHA256 binds both SPAKE2 shared output and the RFC 5705 TLS exporter (`PAIRING_EXPORTER_LABEL`).

### `crates/unify-net`

Asynchronous Tokio transport and local service discovery.

- **Modules:** `discovery`, `transport`.
- **Transport queues:** `spawn_writer` drains the high-priority channel (`HIGH_PRIORITY_CAPACITY = 8_192`, used for `Input`, `Enter`, `Leave`, `Ping`, `Pong`) ahead of the low-priority channel (`LOW_PRIORITY_CAPACITY = 64`, used for `ClipboardBegin` and `ClipboardChunk`), coalescing consecutive `MouseMove` deltas when the socket backpressures.
- **Discovery:** advertises and browses `_unify._tcp.local.` via `mdns-sd`, emitting `DiscoveryEvent::Found` and `DiscoveryEvent::Lost`.

### `crates/unify-platform`

Trait boundary (`Platform`, `Injector`, `StopHandle`) isolating all unsafe FFI and OS event loops.

- **`windows` (`#[cfg(windows)]`):** `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks on a dedicated Win32 message thread, `SendInput` tagged with `INJECTION_MARKER = 0x5546_5949` so Unify ignores its own synthetic events, `AddClipboardFormatListener` (`WM_CLIPBOARDUPDATE`), and exclusion checks for `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`, and `Clipboard Viewer Ignore`.
- **`macos` (`#[cfg(target_os = "macos")]`) & `mac_logic`:** `CGEventTap` on a `CFRunLoop` thread (automatically re-enabled on `TapDisabledByTimeout`), `CGEventPost` on a dedicated injection thread, 250 ms `NSPasteboard.changeCount` polling with `org.nspasteboard.ConcealedType` / `TransientType` filtering, Apple ISO keyboard swap handling, and Accessibility / Input Monitoring queries.
- **`fake`:** deterministic in-memory `Platform` implementation used by `unify-runtime` integration suites.

### `crates/unify-runtime`

Single-task Tokio actor (`Actor::spawn` → `RuntimeHandle`) owning session lifecycle, simultaneous-dial tie-breaking (smaller `device_id` wins), exponential reconnect backoff, pairing state transitions, control hand-over, and `watch::Receiver<StatusSnapshot>` publication.

### `crates/unify-cli` and `app/`

- **`crates/unify-cli`:** terminal binary exposing `RuntimeHandle` commands over `stdin`/`stdout` ([cli-guide.md](cli-guide.md)).
- **`app/` (`src-tauri` + `src`):** Tauri 2 shell (`dev.unify.desktop`) bridging `RuntimeHandle` snapshots and `SettingsPatch` updates over IPC with a strict Content Security Policy (`connect-src ipc: http://ipc.localhost`).

## `config.json` schema reference

Configuration is stored atomically (`config.json.tmp` + `sync_all` + rename). If `config.json` cannot be parsed as JSON, `ConfigStore` renames it to `config.json.corrupt` and writes fresh defaults.

Top-level `ConfigFile` (`CONFIG_SCHEMA_VERSION = 1`):

| Field | Type | Description |
|---|---|---|
| `schema_version` | `u32` | Currently `1`. |
| `settings` | `Settings` | User-configurable runtime settings (table below). |
| `peer` | `PairedPeer \| null` | Pinned peer metadata (`device_id`, `device_name`, `os`, PEM `certificate`, SHA-256 `fingerprint`, `paired_at`, optional `manual_address`, optional `last_address`). |

### `Settings` fields

| Key | Type | Default | Constraints & normalization |
|---|---|---|---|
| `device_name` | `string` | OS hostname or `"Unify"` | Control characters stripped, trimmed, capped at `64` chars; empty falls back to `"Unify"`. |
| `listen_port` | `u16` | `47823` | Values `< 1024` normalize back to `47823`. |
| `peer_side` | `"Left" \| "Right" \| "Top" \| "Bottom"` | `"Right"` | Changing `peer_side` via `SettingsPatch` increments `layout_revision`. |
| `layout_revision` | `u64` | `0` | Monotonic timestamp/counter resolving conflicting layout claims between peers. |
| `input_sharing` | `bool` | `true` | Enables mouse and keyboard forwarding across the active session. |
| `clipboard_sync` | `bool` | `true` | Enables bidirectional plain-text clipboard synchronization. |
| `edge_switch` | `bool` | `true` | Enables pointer-at-edge screen switching. |
| `edge_dwell_ms` | `u32` | `0` | Clamped to `0..=1000` (`MAX_DWELL_MS`). Milliseconds pointer must rest on edge before switching. |
| `corner_guard_px` | `u32` | `0` | Clamped to `0..=200` (`MAX_CORNER_GUARD_PX`). Pixels reserved at each screen corner where edge switching is suppressed. |
| `switch_hotkey` | `bool` | `true` | Enables `Ctrl+Alt+Shift+Space` (`Control+Option+Shift+Space` on macOS). |
| `swap_command_control` | `bool` | `true` | Maps macOS `Command` to `Control` (and `Control` to `Command`) on the sending side when crossing platforms. |
| `invert_scroll` | `bool` | `false` | Reverses vertical and horizontal wheel delta sign when injecting remote scroll events. |
| `mac_keyboard_layout` | `"auto" \| "iso" \| "ansi"` | `"auto"` | Controls Apple ISO top-left / left-Shift key swap detection. |
| `discovery` | `bool` | `true` | Enables local mDNS advertising and browsing. |
| `autostart` | `bool` | `false` | Registers login autostart with `--background` in the desktop shell. |
| `log_level` | `"error" \| "warn" \| "info" \| "debug"` | `"info"` | Runtime diagnostic verbosity. |
| `language` | `"en" \| "tr"` | `"en"` | Desktop UI locale (`en` or `tr`). |
