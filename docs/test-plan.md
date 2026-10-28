# Test plan

## 1. Automated tests (run on the Windows 11 development machine)

| Package | Tests | Coverage |
|---|---|---|
| `unify-proto` | 19 | Serialisation of every message type, split reads, zero/oversized length, malformed and truncated packets, field limits, bidirectional consistency of HID ↔ Windows ↔ macOS tables, Turkish Q key positions, ISO key swap |
| `unify-core` | 56 | Edge-switch state machine (arming, dwell, corner guard, re-entry prevention), remote cursor and return edge, multi-monitor geometry, clipboard loop prevention and re-send, chunk assembly (order, overflow, hash, UTF-8), modifier mapping, hotkey filter, stuck-key release, config load and corrupt-file recovery, language setting default and migration, address validation, backoff |
| `unify-security` | 14 | Identity generation and storage, key/certificate mismatch, identity reset, SPAKE2 right/wrong code, channel binding, reflection and fingerprint-swap attacks, code window expiry and attempt limit |
| `unify-net` | 10 | Prioritised writer (input overtakes clipboard chunks), mouse-move coalescing, queue overflow, malformed frames, mDNS record validation |
| `unify-platform` | 8 + 1 | macOS pure logic (modifier flags, scroll conversion, double click), Windows absolute coordinates and scan code generation; Turkish/CRLF/emoji round trip on the **real Windows clipboard** with `WM_CLIPBOARDUPDATE` notification (`--include-ignored`) |
| `unify-runtime` | 17 | Real TLS between two runtimes: approved pairing, wrong code, local rejection, bidirectional clipboard (including 400 KB) without loops, secret-content filter, clipboard off, edge crossing + Command→Ctrl, rejection of unrequested input, key release on disconnect, hotkey return, refusal when the peer disabled sharing, reconnect after restart, pairing removal, rejection of unknown certificates and pre-authentication messages, RTT measurement |
| `app` (Vitest) | 29 | Spring animator (settling, monotonic, reduced motion), status screen in English and Turkish, settings patch, pair button, permission banner, release control, code input normalisation, English/Turkish key parity, layout drag-and-drop (arrow keys, drop on nearest side, cancel), spring, momentum projection, rubber-band and release-velocity physics |

Commands: `cargo test --workspace`, `cargo test -p unify-platform -- --include-ignored`, `npm --prefix app test`.

Additional verification:

- The macOS layer passes `cargo check -p unify-platform --target aarch64-apple-darwin`. **It was not run on a Mac.**
- Two real `unify-cli` processes paired and connected on this Windows machine using the real Windows Credential Manager, TLS and mDNS (loopback RTT 0–1 ms).
- The Windows installer (`Unify_0.1.0_x64-setup.exe`, 3.39 MB) was produced; the release app was launched and its UI inspected.

## 2. Measured and target performance

| Metric | Target | Measured | Environment |
|---|---|---|---|
| Ping/pong RTT | Wi-Fi p50 < 8 ms, wired < 2 ms | 0–1 ms | Loopback on one machine; **Wi-Fi not measured** |
| `unify.exe` idle memory | < 60 MB working set | 42.1 MB working set, 9.9 MB private | Windows 11, release, window open |
| `unify.exe` idle CPU | < 1 % of one core | No thread exceeded 20 ms over a 20 s sample | Windows 11, release, mouse idle |
| WebView2 memory | — | 6 processes, ~358 MB summed working set (shared pages counted more than once) | Window open; processes stay alive when the window is hidden |
| Pointer latency | Imperceptible | **Not measured**; to be measured on real hardware | — |

## 3. Manual acceptance tests on real hardware

Environment: Apple M2 MacBook (record macOS version), Windows 11 PC, same Wi-Fi or Ethernet. Record the result and versions for every test. Simulated results do not go into this table.

| ID | Steps | Expected | MVP criterion |
|---|---|---|---|
| M1 | Install on the Mac from the DMG and open | Gatekeeper warning, opens via Open Anyway; menu bar icon | 1 |
| M2 | Run the Windows installer | No elevation; firewall prompt; tray icon | 2 |
| M3 | Status screen on the Mac before granting permissions | Permission warning; keyboard/mouse sharing not shown as working | 10 |
| M4 | Grant Accessibility and Input Monitoring | Warning disappears within 3 s; capture running | 10 |
| M5 | Create code on Windows, pick Windows from the mDNS list on the Mac and enter the code | Approval dialog on Windows; fingerprints match; both sides connected after approval | 3 |
| M6 | Try a wrong code | "Code could not be verified"; no pairing | 3 |
| M7 | Copy multi-line Turkish text on the Mac, Ctrl+V on Windows | Identical text | 4, 8 |
| M8 | Copy on Windows, Command+V on the Mac | Identical text | 5, 8 |
| M9 | Copy 5 MB of text while moving the mouse | Pointer stays smooth; text arrives complete | 4 |
| M10 | Copy a password from a password manager | Not sent; "last skipped: secret" | Security |
| M11 | Mac on the left, Windows on the right; move the Mac pointer to the right edge | Pointer appears on the left edge of Windows at the same height | 6, 7 |
| M12 | Type `ğüşıöçİ` and Shift capitals in Notepad with Turkish Q | Correct characters | 8 |
| M13 | Command+C / Command+V / Command+Z from the Mac keyboard | Ctrl+C / Ctrl+V / Ctrl+Z on Windows | 8 |
| M14 | Right Option + Q (`@` on Windows TR-Q) | `@` | 8 |
| M15 | Left, right, middle click, double click, drag within a screen, wheel, horizontal scroll, back/forward buttons | All correct; double-click selection works | 6 |
| M16 | Move the pointer back over the left edge of Windows | Control returns to the Mac; no sticking or ping-pong | 7 |
| M17 | Reverse direction: control the Mac from the Windows keyboard and mouse (mirror of M11–M16) | Same results; double click works on the Mac | 6 |
| M18 | While Windows is controlled hold Shift and turn off Wi-Fi on the Mac | Shift released on Windows within 5 s; Mac regains control | 9 |
| M19 | Turn Wi-Fi back on | Reconnects without user action | 9 |
| M20 | Sleep the Mac, wake it after 2 minutes | Reconnects | 9 |
| M21 | Restart Windows (autostart on) | Unify starts in the background after sign-in and connects | 9 |
| M22 | While Windows is controlled press `Control+Option+Shift+Space` on the Mac | Control returns to the Mac immediately | 9 |
| M23 | Try typing into an elevated terminal on Windows | Not delivered; documented limitation | 10 |
| M24 | Lock Windows (Win+L) and type from the Mac | Lock screen cannot be controlled; documented limitation | 10 |
| M25 | Attach a second monitor to the Mac, check the layout page | Displays listed; switching uses the outermost edge | 7 |
| M26 | Set Windows scaling to 150 % | Crossing points at the correct position | 7 |
| M27 | Remove the pairing on one side | Disconnects and does not reconnect | Security |
| M28 | Dark/light theme, keyboard navigation, VoiceOver/Narrator on the main screen | Legible, all controls reachable | 11 |
| M29 | Judge pointer latency subjectively, note the RTT shown on the network page | Imperceptible; RTT recorded | Performance |
| M30 | On the Display layout page drag the other computer to each side with mouse and trackpad, flick it, and use the arrow keys | Box follows the pointer 1:1, settles on the nearest side without jumping; the other computer shows the mirrored side | 7, 11 |
| M31 | Switch General → Language between English and Türkçe | Window and tray menu change language immediately; setting survives restart | 11 |

## 4. Not tested

- All macOS runtime behaviour (permission flow, `CGEventTap`, event generation, `NSPasteboard`, Retina coordinates, ISO detection) was **not executed** in this environment; only compiler type checking was done.
- End-to-end behaviour of the real Windows low-level hooks and `SendInput` was not exercised by automated tests (it would affect the user's input); verify with M11–M17.
- Wi-Fi latency, sleep/wake and network changes must be measured on real hardware.
