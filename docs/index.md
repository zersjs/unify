# Unify documentation

Unify shares one keyboard, mouse, and plain-text clipboard between an Apple Silicon Mac and a Windows 11 PC over the local network. When the pointer crosses the configured screen edge, input moves to the paired computer; text copied on one computer can be pasted on the other.

- Direct peer-to-peer connection over TLS 1.3 with mutual certificate authentication
- One-time 6-digit pairing via SPAKE2 bound to the TLS 1.3 session exporter
- No cloud service, no user account, no telemetry, and no clipboard history
- Shared Rust workspace (`unify-proto` through `unify-runtime`), native OS hooks, CLI prototype (`unify-cli`), and Tauri 2 desktop shell (`app/`)

## Project status

| Component | Current state |
|---|---|
| Rust core & protocol (`crates/*`) | Implemented; automated unit and integration tests pass on Windows 11 |
| Windows 11 native layer & installer | Compiled, packaged (`Unify_0.1.0_x64-setup.exe`, unsigned), and tested locally |
| macOS Apple Silicon native layer | Implemented and type-checked for `aarch64-apple-darwin`; awaiting execution on physical Mac hardware |
| Security review | Threat model documented; no independent third-party security audit has been performed |
| Hardware acceptance suite | Pending physical two-machine run ([test-plan.md](test-plan.md)) |

## Getting started

- [Quick start](quick-start.md) — connect a Mac and a Windows 11 PC in five steps, test edge switching, and verify the emergency release shortcut.
- [Installation, building and removal](installation-and-build.md) — installer packages, building from source with Cargo and Tauri, code-signing state, and clean uninstallation.
- [Operating system permissions](permissions.md) — macOS Accessibility, Input Monitoring, and Local Network grants; Windows Firewall rules and UIPI boundaries.
- [Secure device pairing](pairing.md) — how the 6-digit SPAKE2 exchange, TLS channel binding, and SHA-256 fingerprint pinning work.

## Operations and security

- [Network and firewall](network-and-firewall.md) — TCP port `47823`, UDP port `5353` mDNS discovery (`_unify._tcp.local.`), manual IP fallback, and guest Wi-Fi isolation.
- [Threat model](threat-model.md) — assets protected, attacker capabilities on shared LANs, mitigations, and accepted residual risks.
- [Troubleshooting](troubleshooting.md) — symptoms, diagnostic checks, and log locations on Windows and macOS.
- [Known limitations](known-limitations.md) — two-machine limit, plain-text-only clipboard, Windows UAC/lock-screen boundary, and unsigned package caveats.

## Engineering reference

- [CLI prototype guide (`unify-cli`)](cli-guide.md) — command-line flags, interactive terminal commands, and single-machine multi-profile testing.
- [Architecture and protocol](architecture-and-protocol.md) — thread model, wire protocol version 1, connection state machine, and edge hand-over rules.
- [Crate and configuration reference](crate-reference.md) — module breakdown across all seven Rust crates, wire constants, and `config.json` schema fields.
- [Technical assessment and decision record](technical-assessment.md) — platform constraints, architecture trade-offs, and MVP scope boundaries.
- [Test plan](test-plan.md) — automated test layers and the physical M2 Mac + Windows 11 acceptance checklist.
