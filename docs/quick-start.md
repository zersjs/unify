# Quick start

This guide walks through connecting a Mac (Apple Silicon) and a Windows 11 PC on the same local network using either the `unify-cli` terminal prototype or a locally built desktop package.

## System requirements

| Requirement | macOS | Windows |
|---|---|---|
| Operating system | macOS 13.0 or newer (Apple Silicon `aarch64`) | Windows 11 (`x86_64`) |
| Network | Same local subnet (Wi-Fi or Ethernet) without client isolation | Private network profile in Windows Settings |
| Inbound port | TCP `47823` (configurable) | TCP `47823` allowed for Private networks |
| Optional discovery | UDP `5353` multicast (`_unify._tcp.local.`) | UDP `5353` multicast (`_unify._tcp.local.`) |

If you are building from source instead of running a CI-built package, install Rust 1.80+ and Node.js 22+ first; see [installation-and-build.md](installation-and-build.md).

## 1. Launch Unify on both computers

To run the command-line prototype directly from a source checkout on each machine:

```bash
cargo run -p unify-cli -- --log info
```

To run the Tauri 2 desktop window in development mode instead:

```bash
npm --prefix app ci
npm --prefix app run tauri dev
```

On first launch, each computer generates a P-256 key pair and a self-signed certificate. The private key is stored in the macOS Keychain or Windows Credential Manager, and non-secret settings are written to `config.json`.

## 2. Grant operating system permissions

### On macOS

macOS blocks synthetic input and global keyboard hooks until explicit user consent is granted:

1. Open **System Settings → Privacy & Security → Accessibility** and enable Unify (or your terminal emulator when running `unify-cli`).
2. Open **System Settings → Privacy & Security → Input Monitoring** and enable the same binary.
3. On macOS 15 or newer, allow the **Local Network** prompt when Unify starts listening.

In `unify-cli`, run `permissions` to trigger the system prompt and verify `accessibility=Granted` and `input_monitoring=Granted` via `status`. Full details are in [permissions.md](permissions.md).

### On Windows 11

No administrator elevation is needed. When Windows Defender Firewall prompts on first launch, select **Private networks** and leave **Public networks** unchecked ([network-and-firewall.md](network-and-firewall.md)).

## 3. Pair the two computers

Pairing uses a single-use 6-digit SPAKE2 code valid for 120 seconds.

### Using `unify-cli`

1. On Computer A, start a pairing window:
   ```text
   pair
   ```
   The terminal prints a 6-digit code (for example `482 910`).
2. Check Computer A's local IP address (or view discovered peers on Computer B with `status`).
3. On Computer B, join using Computer A's IP/port and code:
   ```text
   join 192.168.1.24:47823 482910
   ```
4. Computer A displays Computer B's device name, operating system, and SHA-256 certificate fingerprint. Confirm on Computer A:
   ```text
   approve
   ```

### Using the desktop window

1. On Computer A, open **Pair → Create code**.
2. On Computer B, select Computer A from the discovered list (or enter `IP:47823` manually), enter the 6-digit code, and confirm.
3. Click **Approve** on Computer A after verifying the device name.

Once approved, both computers pin each other's SHA-256 certificate fingerprint and establish a mutual TLS 1.3 session automatically ([pairing.md](pairing.md)).

## 4. Set the physical screen side

Tell Unify where the other computer sits physically on your desk so moving the pointer to that edge triggers the hand-over.

In `unify-cli` (if the Windows PC sits to the right of the Mac, run this on the Mac):

```text
side right
```

Valid positions are `left`, `right`, `top`, and `bottom`. Updating the side increments `layout_revision` and synchronizes the complementary layout claim to the connected peer automatically (`Right` on the Mac maps to `Left` on the Windows PC).

## 5. Test input hand-over, clipboard sync, and emergency release

1. **Emergency release first:** press `Ctrl+Alt+Shift+Space` on Windows (`Control+Option+Shift+Space` on macOS). Whenever Unify is forwarding your keyboard and mouse to the remote screen, this shortcut returns control to your local computer immediately and releases any held keys.
2. **Edge switch:** move the pointer to the configured edge. Control transfers to the paired computer at the matching vertical or horizontal ratio. Move back across the opposite edge on the remote screen to return.
3. **Plain-text clipboard:** copy a line of UTF-8 text on one machine (`Cmd+C` or `Ctrl+C`) and paste on the other (`Ctrl+V` or `Cmd+V`). Note that `swap_command_control` is enabled by default so `Cmd+C` on a Mac keyboard sends `Ctrl+C` when controlling Windows.

If discovery does not find the peer on restricted Wi-Fi, set a direct address with `address 192.168.1.24:47823` or consult [troubleshooting.md](troubleshooting.md).
