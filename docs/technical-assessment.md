# Technical assessment and decision record

Unify is a personal desktop application that shares keyboard, mouse and text clipboard between an Apple Silicon MacBook and a Windows 11 PC over the local network. It has no cloud, account, telemetry or AI component.

## 1. Technical obstacles

| Area | macOS | Windows 11 |
|---|---|---|
| Capturing input | Requires `CGEventTap`. Swallowing events needs an active (filtering) tap and the **Accessibility** permission; listening needs **Input Monitoring**. The system disables a tap whose callback is slow; it must be re-enabled. | `SetWindowsHookEx(WH_KEYBOARD_LL / WH_MOUSE_LL)` works without elevation. A callback that exceeds `LowLevelHooksTimeout` is skipped. |
| Generating input | `CGEventPost` needs Accessibility. With unsigned/ad-hoc builds every rebuild may invalidate the grant. | `SendInput`. UIPI blocks input to elevated windows, the UAC secure desktop and the lock screen. This is an OS boundary and is not bypassed. |
| Clipboard | No change notification API exists; `NSPasteboard.changeCount` is read every 250 ms (one integer comparison). | Event-driven `WM_CLIPBOARDUPDATE` via `AddClipboardFormatListener`. |
| Local network | macOS 15+ requires the Local Network privacy permission; `Info.plist` must contain `NSLocalNetworkUsageDescription` and `NSBonjourServices`. | Windows Firewall prompts on first listen; the Public profile may block inbound connections. |
| Pointer while remote | The pointer is parked; a background app cannot hide the system pointer through public API. | Same: the pointer is parked at the centre of the primary display and remains visible. |
| Keyboard layout | The physical key position (USB HID usage) is transmitted; the receiver's layout produces the character. On Apple ISO keyboards the top-left key and the key next to left Shift have swapped virtual codes; ISO/ANSI is detected. | Scan codes (set 1, `E0` extended) are mapped to HID. |

## 2. Selected architecture

**Shared Rust core + Tauri 2 shell + platform-specific native input/clipboard layer.**

- Networking, protocol, pairing, TLS, state machines and clipboard sync are written once in Rust and verified by unit and integration tests on Windows.
- Input capture/injection and clipboard access call OS APIs directly (`windows-sys`, `core-graphics`, `objc2-app-kit`). The UI technology is not on the latency path: a mouse event travels from the hook to the socket through Rust code only.
- The UI runs in the system web view (WKWebView on macOS, WebView2 on Windows). The package is about ten times smaller than Electron and ships no Chromium.

### Alternatives considered

| Option | Pros | Cons | Decision |
|---|---|---|---|
| SwiftUI (Mac) + WinUI 3 (Windows) + Rust core over FFI | Most native look | Two UI code bases and two FFI layers; the SwiftUI half cannot be compiled or verified in this Windows development environment | Rejected: maintenance cost and share of unverifiable code |
| Tauri 2 + Rust | One UI code base, system web view, direct Rust integration, tray/menu bar and autostart support | UI is not 100 % native controls; platform feel comes from CSS tokens | **Selected** |
| Electron | Mature ecosystem | ~150 MB package, bundled Chromium, extra memory; the input layer would still need native modules | Rejected: conflicts with the low-footprint goal |

## 3. Security decisions

- **Transport:** TLS 1.3 (rustls with the ring provider), mutual authentication. Every device generates its own P-256 key and self-signed certificate. After pairing, the peer certificate is pinned by its SHA-256 fingerprint. A connection presenting an unpinned certificate is closed before any protocol message is processed.
- **Pairing:** single-use 6-digit code + SPAKE2 (password-authenticated key exchange). The code never crosses the network and offline brute force is impossible. The confirmation MAC is bound to the TLS exporter (RFC 5705); a man in the middle would terminate two different TLS sessions and fail confirmation. The code is valid for 120 seconds and is revoked after 5 failed attempts. The user on the code-generating computer approves explicitly.
- **Replay:** TLS 1.3 record sequence numbers and fresh per-session keys; 0-RTT and session tickets are disabled.
- **Key storage:** private key in the macOS Keychain / Windows Credential Manager (`keyring`). The certificate (public) lives in the configuration folder.
- **Accepting input:** remote input is applied only on an authenticated session and only after this computer accepted control (`Enter`).
- **Privacy:** clipboard content and key codes are never logged; no clipboard history is kept. Content marked secret by password managers (`ExcludeClipboardContentFromMonitorProcessing`, `org.nspasteboard.ConcealedType`) is not sent.

## 4. MVP scope

Included: secure pairing, TLS connection, automatic reconnect, mDNS discovery plus manual IP, bidirectional text clipboard, bidirectional keyboard/mouse forwarding, edge-switch state machine, hotkey switch and emergency return (`Ctrl+Alt+Shift+Space`; `Control+Option+Shift+Space` on Mac), release of held keys on disconnect, Turkish UI, tray/menu bar, autostart (off by default).

Out of scope (phase 5): image/file/rich-text clipboard, cross-screen drag and drop, media keys (HID consumer page), character-based layout translation, more than two computers.

## 5. Permissions

- macOS: Accessibility (generate input, swallow events), Input Monitoring (listen to keyboard), Local Network (macOS 15+). See [permissions.md](permissions.md).
- Windows: no elevation required. A firewall exception (Private network) is required.

## 6. What must be verified on real hardware

The development environment is Windows 11. The macOS code passes `cargo check --target aarch64-apple-darwin` but cannot run here. The following can only be verified on a real M2 Mac and Windows 11 PC; see [test-plan.md](test-plan.md):

1. macOS permission flow and event swallowing via `CGEventTap`.
2. Correct characters from generated key events on Turkish Q (including the ISO key swap).
3. Natural-feeling scroll direction and speed in both directions.
4. Real latency over Wi-Fi (the app shows the RTT measured with ping/pong).
5. Reconnection after sleep/wake and network changes.
6. Accuracy of crossing points between Retina points and Windows physical pixels.
