use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, BufReader};
use unify_core::config::SettingsPatch;
use unify_platform::PermissionKind;
use unify_proto::Side;
use unify_runtime::{ConnectionState, ControlState, PairingStatus, RuntimeHandle, RuntimeOptions, StatusSnapshot};
use unify_security::{OsSecretStore, SecretStore};

fn parse_side(value: &str) -> Option<Side> {
    match value {
        "left" => Some(Side::Left),
        "right" => Some(Side::Right),
        "top" => Some(Side::Top),
        "bottom" => Some(Side::Bottom),
        _ => None,
    }
}

#[derive(Parser, Debug)]
#[command(name = "unify-cli", version, about = "Unify technical prototype: secure keyboard, mouse and clipboard sharing")]
struct Args {
    #[arg(long, default_value = "default")]
    profile: String,
    #[arg(long)]
    config_dir: Option<PathBuf>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    no_discovery: bool,
    #[arg(long, default_value = "info")]
    log: String,
}

fn default_config_dir(profile: &str) -> PathBuf {
    let base = directories::ProjectDirs::from("dev", "unify", "Unify")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("unify"));
    if profile == "default" {
        base
    } else {
        base.join(format!("profile-{profile}"))
    }
}

fn describe(status: &StatusSnapshot) -> String {
    let connection = match status.connection.state {
        ConnectionState::NotPaired => "Pairing required",
        ConnectionState::Waiting => "Waiting for connection",
        ConnectionState::Connecting => "Connecting",
        ConnectionState::Reconnecting => "Reconnecting",
        ConnectionState::Connected => "Connected",
    };
    let control = match status.control {
        ControlState::Local => "control is on this computer",
        ControlState::Controlling => "this keyboard/mouse controls the other computer",
        ControlState::Controlled => "the other computer controls this one",
    };
    let peer = status.peer.as_ref().map_or_else(|| String::from("none"), |p| format!("{} ({})", p.name, p.os));
    let rtt = status.connection.rtt_ms.map_or_else(String::new, |rtt| format!(", latency {rtt} ms"));
    format!("[status] {connection}{rtt} | peer: {peer} | {control}")
}

fn describe_pairing(pairing: &PairingStatus) -> String {
    match pairing {
        PairingStatus::Hosting { code, .. } => format!("[pairing] Code: {} {} — on the other computer run: join <address> {code}", &code[..3], &code[3..]),
        PairingStatus::Joining { address } => format!("[pairing] connecting to {address}"),
        PairingStatus::AwaitingApproval { peer_name, peer_os, fingerprint } => {
            format!("[pairing] {peer_name} ({peer_os}) wants to pair. Fingerprint {fingerprint}. Accept: approve | Decline: reject")
        }
        PairingStatus::AwaitingPeerApproval { peer_name } => format!("[pairing] waiting for approval on {peer_name}"),
        PairingStatus::Succeeded { peer_name } => format!("[pairing] paired with {peer_name}"),
        PairingStatus::Failed { reason } => format!("[pairing] failed: {reason}"),
    }
}

fn print_help() {
    println!(
        "Commands:
  status                        detailed status
  pair                          create a pairing code
  join <address> <code>         join the code shown on the other computer
  approve | reject              accept or decline an incoming pairing request
  side <left|right|top|bottom>  position of the other computer
  address <host:port>           manual address (empty: remove)
  input <on|off>                keyboard/mouse sharing
  clipboard <on|off>            clipboard sync
  resync                        send the clipboard again
  release                       take back control
  permissions                   request operating system permissions
  unpair                        remove the pairing
  quit                          exit"
    );
}

async fn run_command(handle: &RuntimeHandle, line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let result = match parts.as_slice() {
        [] => Ok(()),
        ["help"] => {
            print_help();
            Ok(())
        }
        ["quit"] | ["exit"] => return false,
        ["status"] => {
            let status = handle.status();
            println!("{}", describe(&status));
            println!(
                "  device: {} [{}] port {} fingerprint {} key storage {}",
                status.device.name, status.device.os, status.device.port, status.device.fingerprint, status.device.secret_backend
            );
            println!(
                "  permissions: accessibility={} input_monitoring={} | capture={:?} | clipboard={:?}",
                status.permissions.accessibility, status.permissions.input_monitoring, status.capture, status.clipboard.activity
            );
            for device in &status.discovered {
                println!("  discovered: {} ({}) {}", device.name, device.os, device.address);
            }
            if let Some(error) = &status.last_error {
                println!("  last error: {}", error.code);
            }
            Ok(())
        }
        ["pair"] => handle.start_pairing_host().await,
        ["join", address, code] => handle.join_pairing((*address).to_owned(), (*code).to_owned()).await,
        ["approve"] => handle.approve_pairing(true),
        ["reject"] => handle.approve_pairing(false),
        ["side", side] => match parse_side(side) {
            Some(side) => handle.update_settings(SettingsPatch { peer_side: Some(side), ..Default::default() }).await,
            None => {
                println!("Invalid side: left, right, top or bottom");
                Ok(())
            }
        },
        ["address"] => handle.set_manual_address(None).await,
        ["address", address] => handle.set_manual_address(Some((*address).to_owned())).await,
        ["input", value] => handle.update_settings(SettingsPatch { input_sharing: Some(*value == "on"), ..Default::default() }).await,
        ["clipboard", value] => handle.update_settings(SettingsPatch { clipboard_sync: Some(*value == "on"), ..Default::default() }).await,
        ["resync"] => handle.resync_clipboard(),
        ["release"] => handle.release_control(),
        ["permissions"] => {
            let request = handle.request_permissions();
            let _ = handle.open_permission_settings(PermissionKind::Accessibility);
            request
        }
        ["unpair"] => handle.remove_peer().await,
        _ => {
            println!("Unknown command. Type 'help'.");
            Ok(())
        }
    };
    if let Err(err) = result {
        println!("[error] {}", err.code);
    }
    true
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_new(&args.log).unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with_writer(std::io::stderr)
        .init();
    let platform = match unify_platform::native() {
        Ok(platform) => platform,
        Err(err) => {
            eprintln!("This platform is not supported: {err}");
            std::process::exit(2);
        }
    };
    let config_dir = args.config_dir.clone().unwrap_or_else(|| default_config_dir(&args.profile));
    let secrets: Arc<dyn SecretStore> = Arc::new(OsSecretStore);
    let handle = match unify_runtime::start(RuntimeOptions {
        config_dir,
        profile: args.profile.clone(),
        platform,
        secrets,
        bind_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        listen_port_override: args.port,
        discovery_enabled: !args.no_discovery,
    }) {
        Ok(handle) => handle,
        Err(err) => {
            eprintln!("Failed to start: {}", err.code);
            std::process::exit(1);
        }
    };
    println!("Unify prototype started. Type 'help' for commands.");
    let mut status_rx = handle.subscribe();
    let mut last_line = String::new();
    let mut last_pairing: Option<PairingStatus> = None;
    let mut last_error: Option<&'static str> = None;
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        tokio::select! {
            changed = status_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let status = status_rx.borrow_and_update().clone();
                let line = describe(&status);
                if line != last_line {
                    println!("{line}");
                    last_line = line;
                }
                if status.pairing != last_pairing {
                    if let Some(pairing) = &status.pairing {
                        println!("{}", describe_pairing(pairing));
                    }
                    last_pairing = status.pairing.clone();
                }
                let error = status.last_error.as_ref().map(|e| e.code);
                if error != last_error {
                    if let Some(code) = error {
                        println!("[warning] {code}");
                    }
                    last_error = error;
                }
            }
            line = lines.next_line() => match line {
                Ok(Some(line)) => {
                    if !run_command(&handle, line.trim()).await {
                        break;
                    }
                }
                _ => break,
            },
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    handle.shutdown().await;
    println!("Unify stopped.");
}
