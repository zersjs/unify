# Unify

[![Türkçe](https://img.shields.io/badge/Dokümanlar-Türkçe-c8102e?style=for-the-badge)](docs/tr/README.md)

**English** · [Türkçe](docs/tr/README.md)

<img width="1018" height="588" alt="image" src="https://github.com/user-attachments/assets/9577e524-9530-48cd-aff1-d0e667f1c90c" />

Unify shares one **keyboard, mouse and text clipboard** between a macOS (Apple Silicon) computer and a Windows 11 computer over the local network. Move the pointer past the edge of one screen and control moves to the other computer; copy text on one machine and paste it on the other.

- End-to-end TLS 1.3 with certificate pinning, and code-based pairing over SPAKE2
- No accounts, cloud services or telemetry; works without internet access
- Rust core, native OS input APIs, Tauri 2 user interface

## Status

| Component | State |
|---|---|
| Windows 11 | Builds, installer produced, automated tests pass, application launched and inspected |
| macOS (M2) | Implemented and type-checked for `aarch64-apple-darwin`; **not yet run on a real Mac** |
| Hardware acceptance tests | Pending ([test plan](docs/test-plan.md)) |

## Documentation

1. [Technical assessment and decision record](docs/technical-assessment.md)
2. [Architecture and protocol](docs/architecture-and-protocol.md)
3. [Threat model](docs/threat-model.md)
4. [Installation, building and removal](docs/installation-and-build.md)
5. [Operating system permissions](docs/permissions.md)
6. [Secure device pairing](docs/pairing.md)
7. [Network and firewall](docs/network-and-firewall.md)
8. [Troubleshooting](docs/troubleshooting.md)
9. [Test plan](docs/test-plan.md)
10. [Known limitations](docs/known-limitations.md)

## Quick start

The interface is available in English (default) and Turkish (**General → Language**).

1. Install Unify on both computers ([installation](docs/installation-and-build.md)).
2. On the Mac, grant Accessibility and Input Monitoring ([permissions](docs/permissions.md)).
3. On one computer choose **Pair → Create code**, on the other **Join with code** ([pairing](docs/pairing.md)).
4. On the **Display layout** page, drag the other computer to the side where it physically stands.

Emergency return shortcut: Windows `Ctrl+Alt+Shift+Space`, Mac `Control+Option+Shift+Space`.

## Repository layout

```
crates/unify-proto      wire messages, framing, key code tables
crates/unify-core       config, geometry, edge-switch state machine, clipboard sync
crates/unify-security   device identity, key storage, TLS pinning, SPAKE2 pairing
crates/unify-net        TLS transport, prioritised writer, mDNS discovery
crates/unify-platform   Windows and macOS input, clipboard and display integration
crates/unify-runtime    actor that ties everything together
crates/unify-cli        terminal prototype
app/                    Tauri 2 shell (src-tauri) and React UI (src)
docs/                   English documentation (Turkish in docs/tr)
```

## Building

```bash
cargo test --workspace
```

```bash
npm --prefix app ci
```

```bash
npm --prefix app run tauri build -- --bundles nsis
```

See [installation-and-build.md](docs/installation-and-build.md) for macOS packaging, signing and CI.

## License

MIT
