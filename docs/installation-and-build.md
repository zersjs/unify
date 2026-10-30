# Installation, building and removal

## Installing

### Windows 11

1. Run `Unify_0.1.0_x64-setup.exe`. It installs for the current user only and does not request elevation.
2. The package is unsigned, so SmartScreen may show "Windows protected your PC": **More info → Run anyway**. Removing this warning requires a code-signing certificate.
3. On first launch Windows Firewall asks for access: tick **Private networks** only ([network-and-firewall.md](network-and-firewall.md)).
4. WebView2 ships with Windows 11; if missing, the installer downloads Microsoft's bootstrapper.

### macOS (Apple Silicon)

1. Open `Unify_0.1.0_aarch64.dmg` and drag Unify into **Applications**.
2. The app is not signed with an Apple Developer ID and not notarized (ad-hoc signature only). Gatekeeper blocks the first launch: **System Settings → Privacy & Security → Open Anyway**. On macOS 15 the right-click → Open shortcut no longer bypasses this; use the settings page.
3. Grant permissions ([permissions.md](permissions.md)). Unify lives in the menu bar and has no Dock icon.

### Autostart

Off **by default**. Enable it under **General → Start at login**. When enabled, the app starts with the `--background` argument and does not show its window. Windows uses the per-user `Run` registry key; macOS uses a LaunchAgent in `~/Library/LaunchAgents`.

### Updates

Unify checks `latest.json` on the newest GitHub release 20 seconds after launch and then hourly. It also checks when the paired device reports a different protocol version. An update is installed only if its minisign signature matches the public key built into the app.

With **General → Update automatically** on (the default), Unify installs while control is local and screen view is idle, then restarts. With it off, **General → Check now** and **Install and restart** do the same by hand. Development builds never check.

The check sends an ordinary HTTPS request to GitHub, which sees the IP address and app version. Turn the setting off to stop automatic installs; hourly checks continue so the app can show that an update exists.

## Development environment

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.80 or newer (developed with 1.98) | `rustup` |
| Node.js | 22 or newer | `npm` |
| Windows | Visual Studio Build Tools (MSVC, Windows SDK) | needed to compile `ring` |
| macOS | Xcode Command Line Tools (`xcode-select --install`) | |

```bash
npm --prefix app ci
```

## Commands

Rust tests (repository root):

```bash
cargo test --workspace
```

Frontend type check and tests:

```bash
npm --prefix app run typecheck
```

```bash
npm --prefix app test
```

Technical prototype (terminal):

```bash
cargo run -p unify-cli -- --log info
```

Desktop app in development mode:

```bash
npm --prefix app run tauri dev
```

Windows installer:

```bash
npm --prefix app run tauri build -- --bundles nsis
```

macOS bundle (on a Mac):

```bash
npm --prefix app run tauri build -- --bundles app,dmg
```

Outputs are written to `target/release/bundle/`. On Windows the macOS code can only be type-checked:

```bash
cargo check -p unify-platform --target aarch64-apple-darwin
```

When pushed to GitHub, `.github/workflows/ci.yml` runs lint, type checks and tests on both platforms, then produces unsigned `.exe` and `.dmg` packages as artifacts. This is the shortest way to obtain a macOS package without building on a Mac.

## Releases

After `ci` succeeds on a push to `main`, `.github/workflows/release.yml` publishes version `0.2.<run number>` if `crates/`, `app/` or the Cargo files changed since the last tag. The version is stamped during the build and is not committed. Use **Run workflow** on the Actions page to force a release.

Repository secrets:

| Secret | Contents |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Update signing private key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password of that key |
| `UNIFY_MAC_CERT_P12` | Base64 of the macOS self-signed `.p12` |
| `UNIFY_MAC_CERT_PASSWORD` | Password of that `.p12` |

Losing the update key means installed copies can no longer accept updates. Rotating the macOS certificate means macOS permissions must be granted again.

## Signing and notarization

| Platform | Requirement | State |
|---|---|---|
| macOS | Apple Developer Program membership (annual), "Developer ID Application" certificate, notarization with `notarytool` | Not done. Release builds use a stable self-signed certificate (`Unify Self-Signed`) so that privacy permissions can survive updates; this is not yet confirmed on a Mac. CI artifacts remain ad-hoc signed and lose permissions on each build. Gatekeeper still blocks the first manual install. |
| Windows | Authenticode code-signing certificate (OV/EV) | Not done; a SmartScreen warning is expected. |

To sign, fill in `bundle.macOS.signingIdentity` and, for Windows, `bundle.windows.certificateThumbprint` in `tauri.conf.json`.

## Removal and cleanup

1. In Unify choose **Paired device → Remove pairing** (do it on the other computer too).
2. Uninstall: Windows **Settings → Apps → Unify → Uninstall**; on macOS move Unify to the Trash.
3. Delete settings and identity files:
   - Windows: `%APPDATA%\dev.unify.desktop`, `%LOCALAPPDATA%\dev.unify.desktop`
   - macOS: `~/Library/Application Support/dev.unify.desktop`, `~/Library/Logs/dev.unify.desktop`
4. Delete the private key:
   - Windows: **Credential Manager → Windows Credentials**, entry `device-key-default-….dev.unify.desktop`
   - macOS: **Keychain Access**, item with service name `dev.unify.desktop`
5. macOS: remove Unify from Accessibility, Input Monitoring and Local Network under **Privacy & Security**. If autostart was on and the Unify entry in `~/Library/LaunchAgents` remains, delete it manually.
6. Windows: delete the `unify.exe` rule under **Windows Defender Firewall → Advanced settings → Inbound Rules**.
