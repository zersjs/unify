use std::fmt::Write as _;
use std::process::Command;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use unify_core::config::SettingsPatch;
use unify_platform::PermissionKind;
use unify_runtime::{RuntimeError, StatusSnapshot};

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
}

impl From<RuntimeError> for CommandError {
    fn from(error: RuntimeError) -> Self {
        Self { code: error.code }
    }
}

impl CommandError {
    const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

type CommandResult<T = ()> = Result<T, CommandError>;

pub fn apply_autostart(app: &AppHandle, enabled: bool) -> CommandResult {
    let launcher = app.autolaunch();
    let current = launcher.is_enabled().unwrap_or(!enabled);
    if current == enabled {
        return Ok(());
    }
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|err| {
        tracing::warn!(error = %err, "autostart change failed");
        CommandError::new("autostart_failed")
    })
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> StatusSnapshot {
    state.runtime.status()
}

#[tauri::command]
pub async fn update_settings(app: AppHandle, state: State<'_, AppState>, patch: SettingsPatch) -> CommandResult {
    let autostart = patch.autostart;
    let log_level = patch.log_level;
    state.runtime.update_settings(patch).await?;
    if let Some(level) = log_level {
        state.log.set_level(level);
    }
    if let Some(enabled) = autostart {
        if let Err(err) = apply_autostart(&app, enabled) {
            let _ = state.runtime.update_settings(SettingsPatch { autostart: Some(!enabled), ..Default::default() }).await;
            return Err(err);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn start_pairing(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.start_pairing_host().await?)
}

#[tauri::command]
pub fn cancel_pairing(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.cancel_pairing()?)
}

#[tauri::command]
pub async fn join_pairing(state: State<'_, AppState>, address: String, code: String) -> CommandResult {
    if address.len() > 260 || code.len() > 32 {
        return Err(CommandError::new("address_invalid"));
    }
    Ok(state.runtime.join_pairing(address, code).await?)
}

#[tauri::command]
pub fn approve_pairing(state: State<'_, AppState>, accept: bool) -> CommandResult {
    Ok(state.runtime.approve_pairing(accept)?)
}

#[tauri::command]
pub async fn remove_peer(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.remove_peer().await?)
}

#[tauri::command]
pub async fn set_manual_address(state: State<'_, AppState>, address: Option<String>) -> CommandResult {
    if address.as_ref().is_some_and(|value| value.len() > 260) {
        return Err(CommandError::new("address_invalid"));
    }
    Ok(state.runtime.set_manual_address(address).await?)
}

#[tauri::command]
pub fn reconnect_now(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.reconnect_now()?)
}

#[tauri::command]
pub fn resync_clipboard(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.resync_clipboard()?)
}

#[tauri::command]
pub fn release_control(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.release_control()?)
}

#[tauri::command]
pub fn request_permissions(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.request_permissions()?)
}

#[tauri::command]
pub fn open_permission_settings(state: State<'_, AppState>, kind: String) -> CommandResult {
    let kind = match kind.as_str() {
        "accessibility" => PermissionKind::Accessibility,
        "input_monitoring" => PermissionKind::InputMonitoring,
        "local_network" => PermissionKind::LocalNetwork,
        _ => return Err(CommandError::new("unknown")),
    };
    Ok(state.runtime.open_permission_settings(kind)?)
}

#[tauri::command]
pub async fn reset_identity(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.reset_identity().await?)
}

#[tauri::command]
pub fn clear_error(state: State<'_, AppState>) -> CommandResult {
    Ok(state.runtime.clear_error()?)
}

#[tauri::command]
pub fn open_log_folder(state: State<'_, AppState>) -> CommandResult {
    let program = if cfg!(target_os = "macos") { "/usr/bin/open" } else { "explorer.exe" };
    Command::new(program).arg(&state.log_dir).spawn().map(|_| ()).map_err(|err| {
        tracing::warn!(error = %err, "could not open log folder");
        CommandError::new("unknown")
    })
}

#[tauri::command]
pub fn diagnostics_report(state: State<'_, AppState>) -> String {
    let status = state.runtime.status();
    let settings = &status.settings;
    let mut report = String::new();
    let _ = writeln!(report, "Unify {} diagnostics report", status.app_version);
    let _ = writeln!(report, "System: {} ({})", status.device.os, std::env::consts::ARCH);
    let _ = writeln!(report, "Device fingerprint: {}", status.device.fingerprint);
    let _ = writeln!(report, "Listening port: {}", status.device.port);
    let _ = writeln!(report, "Key storage: {}", status.device.secret_backend);
    match &status.peer {
        Some(peer) => {
            let _ = writeln!(report, "Peer: {} fingerprint {} manual address {}", peer.os, peer.fingerprint, if peer.manual_address.is_some() { "set" } else { "none" });
        }
        None => {
            let _ = writeln!(report, "Peer: none");
        }
    }
    let _ = writeln!(
        report,
        "Connection: {:?} attempt {} latency {}",
        status.connection.state,
        status.connection.attempt,
        status.connection.rtt_ms.map_or_else(|| String::from("-"), |rtt| format!("{rtt} ms"))
    );
    let _ = writeln!(report, "Control: {:?} | Capture: {:?}", status.control, status.capture);
    let _ = writeln!(
        report,
        "Permissions: accessibility={} input_monitoring={} local_network={}",
        status.permissions.accessibility, status.permissions.input_monitoring, status.permissions.local_network
    );
    let _ = writeln!(report, "Displays: {}", status.displays.iter().map(|d| format!("{}x{}@{},{}", d.width, d.height, d.x, d.y)).collect::<Vec<_>>().join(" "));
    let _ = writeln!(
        report,
        "Settings: side={:?} input={} clipboard={} edge={} dwell={}ms corner={}px hotkey={} cmd/ctrl={} invert_scroll={} mac_layout={:?} iso={} discovery={} autostart={} language={:?}",
        settings.peer_side,
        settings.input_sharing,
        settings.clipboard_sync,
        settings.edge_switch,
        settings.edge_dwell_ms,
        settings.corner_guard_px,
        settings.switch_hotkey,
        settings.swap_command_control,
        settings.invert_scroll,
        settings.mac_keyboard_layout,
        status.iso_keyboard,
        settings.discovery,
        settings.autostart,
        settings.language
    );
    let stats = &status.stats;
    let _ = writeln!(
        report,
        "Counters: sessions={} input_sent={} input_received={} clipboard_sent={} clipboard_received={} rejected={}",
        stats.sessions_established,
        stats.input_events_sent,
        stats.input_events_received,
        stats.clipboard_sent,
        stats.clipboard_received,
        stats.rejected_connections
    );
    let _ = writeln!(report, "Devices discovered: {}", status.discovered.len());
    let _ = writeln!(report, "Last error: {}", status.last_error.as_ref().map_or("-", |error| error.code));
    report
}
