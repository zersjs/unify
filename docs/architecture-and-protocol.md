# Architecture and protocol

## Modules

| Crate / folder | Responsibility | Platform-specific |
|---|---|---|
| `crates/unify-proto` | Message types, framing (4-byte length + postcard), field validation, USB HID ↔ Windows scan code ↔ macOS virtual key tables | No |
| `crates/unify-core` | Configuration (JSON, atomic writes), display geometry, edge-switch state machine, remote cursor, clipboard sync and chunk assembly, key routing / hotkey filter, Command↔Ctrl mapping, retry backoff | No |
| `crates/unify-security` | Device identity (P-256 + self-signed certificate), secret store (Keychain / Credential Manager), TLS 1.3 configurations and certificate pinning, SPAKE2 pairing | No |
| `crates/unify-net` | TLS connect/accept, prioritised frame writer (input before clipboard chunks, consecutive mouse moves coalesced), mDNS discovery | No |
| `crates/unify-platform` | `Platform` trait; `windows/` (low-level hooks, `SendInput`, clipboard listener, monitor list), `macos/` (`CGEventTap`, `CGEventPost`, `NSPasteboard`, permission queries), `fake` (for tests) | Yes |
| `crates/unify-runtime` | Single-task actor: connection lifecycle, pairing flow, control hand-over, clipboard flow, status publication | Through the platform trait |
| `crates/unify-cli` | Technical prototype: runs the same runtime from a terminal | — |
| `app/` | Tauri 2 shell (`src-tauri`) and React UI (`src`) | — |

Dependency direction: `proto ← core ← {security, net, platform} ← runtime ← {cli, app}`. Platform code lives only in `unify-platform`.

## Threads

- **Input hook thread** (Windows: message loop + `WH_KEYBOARD_LL`/`WH_MOUSE_LL`; macOS: `CFRunLoop` + `CGEventTap`). The callback only classifies the event, writes it to a channel and decides whether to swallow it; it performs no network or disk I/O.
- **Clipboard thread** (Windows: message-only window + `WM_CLIPBOARDUPDATE`; macOS: 250 ms `changeCount` check).
- **macOS injection thread**: owns the `CGEventSource` and modifier state.
- **Tokio**: runtime actor, per-connection reader/writer tasks, listener, discovery bridge.

## Protocol (version 1)

Each frame is a big-endian `u32` length followed by a postcard-encoded `Message`. The largest frame is 64 KiB; a zero length, trailing bytes, a malformed field or an out-of-range value closes the connection.

| Message | Direction | Priority | Purpose |
|---|---|---|---|
| `Hello` | both | high | Version, device ID, name, OS, listen port |
| `Layout` | both | high | Display layout claim + revision (newer wins; ties go to the smaller device ID) |
| `Ping` / `Pong` | both | high | 1 s interval; 5 s of silence = dead connection; RTT measurement |
| `Enter` | controller → controlled | high | Entry point (edge + ratio, or centre) |
| `Leave` | controlled → controller | high | Pointer left through the return edge, with ratio |
| `Release` | both | high | Control ended / refused; the receiver releases held keys |
| `Input` | controller → controlled | high | HID key usage, relative mouse motion, button, wheel (120 = one notch) |
| `ClipboardBegin` / `ClipboardChunk` | both | low | Length + SHA-256, then chunks of at most 16 KiB |
| `Goodbye` | both | high | Close reason |
| `PairStart` / `PairReply` / `PairConfirm` / `PairResult` | pairing | high | SPAKE2 messages, MAC confirmations, user approval |

### Connection state machine

```
TLS established ─┬─ outbound session ── send Hello ── AwaitHello ── Hello verified ── Established
                 ├─ inbound ─────────── AwaitHello ─┬─ Hello (pinned fingerprint) ── Established
                 │                                  ├─ PairStart (pairing window open) ── pairing flow
                 │                                  └─ anything else ── Goodbye(ProtocolViolation) + close
                 └─ outbound pairing ── PairStart ── PairReply ── PairConfirm ⇄ ── PairResult ── close
```

- On an unauthenticated connection, `Input`, `Enter` and clipboard messages are never processed.
- If both sides connect simultaneously, both computers apply the same rule: the session dialled by the smaller device ID survives.
- Retry: exponential from 0.5 s to 10 s with ±20 % jitter. When mDNS sees the peer again, a dial happens immediately.
- Sleep detection: a gap of more than 6 s between timer ticks closes the session and reconnects immediately.

### Control hand-over

| State | Event | Result |
|---|---|---|
| Local | Pointer reaches the configured edge (armed, dwell elapsed, outside corner guard, no mouse button held) | `Enter(edge, ratio)`, pointer parked, events swallowed |
| Local | Hotkey | `Enter(Centre)` |
| Controlling | `Leave` | Pointer returns to the edge point at the same ratio; 250 ms cooldown; re-arming required |
| Controlling | Hotkey / disconnect / `Release` | Back to local |
| Controlled | Pointer tries to cross the return edge | `Leave`, held keys released |
| Controlled | `Release` / disconnect / local hotkey | Held keys and buttons released |
| Both send `Enter` simultaneously | — | The smaller device ID remains the controller |

Edge sticking and infinite ping-pong are prevented by the arming rule: after a return, no switch happens until the pointer has left the edge.

### Key mapping

- The wire carries the USB HID usage (page 7); the physical position is preserved and the receiver's layout produces the character.
- Command ↔ Ctrl translation happens on the sending side, according to the sender's setting.
- If a key was held locally when control moved away, its release is delivered to the local OS, so no key gets stuck on the controller.
- On Windows the fake left Ctrl generated by AltGr is swallowed; only right Alt is forwarded.

### Clipboard sync

- Local change → text read → skipped if marked secret, empty or larger than 16 MiB → skipped if equal to the last sent or last received content → sent.
- Received content is verified by SHA-256 and UTF-8, written to the clipboard and recorded as "last received"; the change notification caused by that write is not echoed back.
- The latest copy made while disconnected is held and sent once a session is established.
- Clipboard chunks use the low-priority queue; the writer always drains the input queue first.
