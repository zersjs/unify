# Unify

Keyboard, mouse, and text clipboard sharing between a Mac and a Windows computer on a local network. No cloud, accounts, or telemetry.

> **Status:** Early stage. Core libraries are being validated with unit tests. The macOS code compiles with `cargo check --target aarch64-apple-darwin`, but it has not yet been tested on real hardware. The Tauri UI shell has not been added yet; the currently working entry point is the command-line prototype (`unify-cli`).

## Open Source & Community Contributions

Unify is an open-source project built to make seamless keyboard, mouse, and clipboard sharing between macOS and Windows simple, secure, and accessible.

The project is actively evolving, and there is plenty of room for improvement. Developers and contributors are welcome to explore the codebase, suggest new features, report bugs, improve documentation, and help make Unify more reliable across different hardware and operating system versions.

**Contributions are welcome!** Whether you're fixing a bug, improving performance, refining the user interface, or proposing a new feature, your contributions can help shape the future of Unify.

### Areas for Contribution

- **Cross-platform compatibility:** Improve support for different macOS and Windows configurations.
- **Performance and reliability:** Reduce input latency and improve connection stability.
- **Security:** Review the pairing protocol, encryption, and credential storage.
- **User experience:** Help build a polished, intuitive desktop interface.
- **Testing:** Test on real hardware, identify edge cases, and expand automated test coverage.
- **Documentation:** Improve setup instructions, troubleshooting guides, and developer documentation.

### Development Status

Unify is currently in an early stage of development. Some features are still experimental, and real-world testing is ongoing. Contributions, constructive feedback, and fresh ideas are especially valuable at this stage.

The goal is to build a reliable, privacy-focused, community-driven tool that anyone can inspect, improve, and use without depending on cloud services or third-party accounts.

**Interested in contributing?** Explore the codebase, open an issue, or submit a pull request to help make Unify better.

Unify is released under the **MIT License**, allowing developers to use, modify, and distribute the project under its license terms.

## Features

- Six-digit, single-use pairing code with password-authenticated key exchange using SPAKE2. The code is never transmitted over the network, expires after 120 seconds, and is invalidated after five failed attempts.
- Mutual authentication using TLS 1.3. The paired device's certificate is pinned by its SHA-256 fingerprint.
- Automatic discovery via mDNS and manual address entry.
- Bidirectional keyboard and mouse sharing. The cursor moves to the other computer when it reaches the edge of the screen.
- Bidirectional text clipboard synchronization. Content marked as secret by password managers is not transmitted.
- Emergency control-release shortcut: `Ctrl+Alt+Shift+Space` (on Mac: `Control+Option+Shift+Space`).
- Pressed keys are released when the connection drops, and automatic reconnection is supported.
- Keys are stored in the operating system's secure credential store: macOS Keychain and Windows Credential Manager.

**Out of scope:** Image and file clipboard support, cross-screen drag and drop, media keys, and support for more than two computers.

## Requirements

- Rust 1.80 or later
- Windows 11 or macOS
- Node.js for the UI (`app/`). Not required for the command-line prototype.

## Quick Start

Build and run the project on both computers:

```bash
cargo run -p unify-cli
```

1. On the first computer, enter `pair`. A six-digit code and the computer's address will be displayed.
2. On the second computer, enter `join <address> <code>`. The address must include the port shown by the first computer's `status` command.
3. On the first computer, approve the incoming request by entering `approve`.
4. Set the other computer's position: `side right` (`left`, `right`, `up`, and `down` are also valid).
5. Enable sharing: `input on` and `clipboard on`.

## Commands

| Command | Description |
|---|---|
| `status` | Display detailed status, device information, and permissions |
| `pair` | Generate a pairing code |
| `join <address> <code>` | Join the pairing session on another computer |
| `approve` / `reject` | Approve or reject an incoming pairing request |
| `side <left\|right\|up\|down>` | Set the position of the other computer |
| `address <host:port>` | Set a manual address; leave empty to remove it |
| `input <on\|off>` | Enable or disable keyboard and mouse sharing |
| `clipboard <on\|off>` | Enable or disable clipboard synchronization |
| `resync` | Send the clipboard contents again |
| `release` | Release control and return it to the local computer |
| `permissions` | Request operating system permissions |
| `unpair` | Remove the pairing |
| `quit` | Exit |

### Command-Line Options

| Option | Description |
|---|---|
| `--profile <name>` | Use a separate configuration profile (default: `default`) |
| `--config-dir <path>` | Specify the configuration directory manually |
| `--port <number>` | Specify the listening port |
| `--no-discovery` | Disable mDNS discovery |
| `--log <filter>` | Set the logging level (default: `info`) |

## Permissions

- **macOS:** Accessibility (for generating input and intercepting events) and Input Monitoring (for listening to keyboard events). macOS 15 and later also require Local Network permission. The `permissions` command opens the relevant permission prompts.
- **Windows:** Administrator privileges are not required. Windows Firewall will ask for permission the first time the application listens for connections. The network profile must be set to **Private**.

## Known Limitations

- On Windows, input cannot be sent to elevated (administrator) windows, the UAC secure desktop, or the lock screen. These are operating system restrictions.
- When control switches to the other computer, the cursor is parked. On macOS, a background application cannot hide the cursor.
- ISO and ANSI keyboard layout differences are detected on MacBooks, but ISO keyboards have not yet been tested.

## Architecture

The workspace consists of the following Rust crates:

| Crate | Responsibility |
|---|---|
| `unify-proto` | Network protocol and message types |
| `unify-core` | Configuration and state machines |
| `unify-security` | Pairing, TLS certificates, and credential storage |
| `unify-net` | TLS connections and mDNS discovery |
| `unify-platform` | OS-specific input and clipboard layers (`windows`, `macos`) |
| `unify-runtime` | Runtime that integrates all layers and exposes the command interface |
| `unify-cli` | Command-line prototype |

Mouse events travel entirely through Rust code, from the input hook to the network. The UI layer is not part of the latency-sensitive path.

See [`docs/01-teknik-degerlendirme.md`](docs/01-teknik-degerlendirme.md) for architectural decisions and alternatives.

## Development

Run the tests:

```bash
cargo test -p unify-proto -p unify-core -p unify-security -p unify-net -p unify-platform -p unify-runtime -p unify-cli
```

Check the macOS platform crate:

```bash
cargo check -p unify-platform --target aarch64-apple-darwin
```

For the UI (inside the `app/` directory):

```bash
npm install
npm run typecheck
npm test
```

## License

MIT. See the `LICENSE` file for details.
