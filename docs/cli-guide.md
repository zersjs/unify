# CLI prototype guide (`unify-cli`)

`unify-cli` is the terminal entry point for Unify (`crates/unify-cli/src/main.rs`). It boots the exact same `unify-runtime` actor, native `unify-platform` hooks, mDNS discovery daemon, and TLS 1.3 transport used by the desktop shell, exposing interactive line commands over standard input.

## Launching `unify-cli`

From the repository root:

```bash
cargo run -p unify-cli -- [OPTIONS]
```

Or using a compiled release binary:

```bash
cargo build --release -p unify-cli
./target/release/unify-cli --log info
```

## Command-line flags

| Flag | Default | Description |
|---|---|---|
| `--profile <NAME>` | `default` | Profile identifier. Selects an isolated configuration folder (`profile-<NAME>`) and separate OS secret-store key entry. |
| `--config-dir <PATH>` | OS default | Explicit directory path containing `config.json` and `identity.json`. Overrides the `--profile` directory calculation. |
| `--port <PORT>` | `config.json` (`47823`) | Overrides `settings.listen_port` for the current process without rewriting `config.json` on disk. |
| `--no-discovery` | `false` | Disables UDP multicast mDNS advertising and browsing (`_unify._tcp.local.`). Use with `address <host:port>`. |
| `--log <FILTER>` | `info` | `tracing_subscriber` filter directive written to `stderr` (for example `error`, `warn`, `info`, `debug`, or `unify_net=debug,info`). |

### Exit codes

- `0` — clean shutdown after `quit`, `exit`, EOF on `stdin`, or `Ctrl+C`.
- `1` — runtime startup failure (printed as `Failed to start: <error_code>`).
- `2` — unsupported host operating system (`unify_platform::native()` only supports Windows and macOS).

## Interactive commands

Once started, `unify-cli` reads commands line-by-line from `stdin` while printing asynchronous runtime status transitions to `stdout`.

| Command | Arguments | Behavior |
|---|---|---|
| `help` | none | Prints the built-in command summary. |
| `status` | none | Prints connection state, RTT in milliseconds, peer name/OS, control state, local device port and certificate fingerprint, secret store backend, OS permission state, capture state, clipboard state, discovered mDNS peers, and last error code. |
| `pair` | none | Opens a 120-second SPAKE2 pairing window as host and prints a 6-digit pairing code (`XXX XXX`). |
| `join` | `<address> <code>` | Connects to `<address>` (`host:port`) as initiator and runs SPAKE2 verification with `<code>`. |
| `approve` | none | Accepts a pending pairing request after SPAKE2 MAC verification succeeds on the host. |
| `reject` | none | Declines a pending pairing request and closes the pairing session. |
| `side` | `left \| right \| top \| bottom` | Sets the physical direction of the paired peer relative to this computer and bumps `layout_revision`. |
| `address` | `[host:port]` | Sets `peer.manual_address` when an argument is supplied, or clears the manual address when invoked with no argument. |
| `input` | `on \| off` | Enables or disables keyboard and mouse sharing (`settings.input_sharing`). |
| `clipboard` | `on \| off` | Enables or disables plain-text clipboard synchronization (`settings.clipboard_sync`). |
| `resync` | none | Re-reads the local plain-text clipboard and transmits it to the connected peer. |
| `release` | none | Immediately returns input control to the local computer and releases held remote keys. |
| `permissions` | none | Requests missing OS permissions and opens macOS **Privacy & Security → Accessibility** settings. |
| `unpair` | none | Deletes the pinned peer record (`peer = null` in `config.json`) and closes any active session. |
| `quit` / `exit` | none | Shuts down the runtime actor, releases hooks, and exits with code `0`. |

## Configuration and key storage paths

When `--config-dir` is omitted, `unify-cli` resolves paths through `directories::ProjectDirs::from("dev", "unify", "Unify")`:

| Platform | Default profile directory (`--profile default`) | Named profile (`--profile b`) |
|---|---|---|
| Windows 11 | `%APPDATA%\unify\Unify\config` | `%APPDATA%\unify\Unify\config\profile-b` |
| macOS | `~/Library/Application Support/dev.unify.Unify` | `~/Library/Application Support/dev.unify.Unify/profile-b` |

Note that the Tauri desktop shell (`app/`) uses bundle identifier `dev.unify.desktop` (`%APPDATA%\dev.unify.desktop` on Windows, `~/Library/Application Support/dev.unify.desktop` on macOS). Pass `--config-dir` if you want `unify-cli` to inspect or share a specific directory.

## Two-instance loopback session on one computer

Using `--profile`, `--port`, and `--no-discovery`, you can test pairing, TLS pinning, and state transitions between two local terminals on a single machine. Keep `input off` on loopback sessions so two hooks on the same OS do not feed into each other:

Terminal 1 (profile `alpha`, port `47823`):

```bash
cargo run -p unify-cli -- --profile alpha --port 47823 --no-discovery
```

```text
input off
pair
```

Terminal 2 (profile `beta`, port `47824`):

```bash
cargo run -p unify-cli -- --profile beta --port 47824 --no-discovery
```

```text
input off
join 127.0.0.1:47823 <6-digit-code-from-alpha>
```

Approve in Terminal 1 (`approve`), then set manual loopback addresses so reconnects target the right ports:

- In Terminal 1: `address 127.0.0.1:47824`
- In Terminal 2: `address 127.0.0.1:47823`

Run `status` in either terminal to verify `Connected`, measured loopback RTT, and `Paired` fingerprint state.
