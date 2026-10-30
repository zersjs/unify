#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod logging;
mod tray;
mod updater;

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use unify_runtime::{RuntimeHandle, RuntimeOptions};
use unify_security::{OsSecretStore, SecretStore};

pub const BACKGROUND_ARG: &str = "--background";
const STATUS_EMIT_INTERVAL: Duration = Duration::from_millis(50);

pub struct AppState {
    pub runtime: RuntimeHandle,
    pub log: logging::LogControl,
    pub log_dir: PathBuf,
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = app.path().app_log_dir()?;
    let config_dir = app.path().app_config_dir()?;
    let log = logging::LogControl::init(&log_dir)?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Unify starting");
    let platform = unify_platform::native()?;
    let secrets: Arc<dyn SecretStore> = Arc::new(OsSecretStore);
    let runtime = tauri::async_runtime::block_on(async {
        unify_runtime::start(RuntimeOptions {
            config_dir,
            profile: String::from("default"),
            platform,
            secrets,
            bind_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            listen_port_override: None,
            discovery_enabled: true,
        })
    })
    .map_err(|err| format!("runtime start failed: {}", err.code))?;
    let initial = runtime.status();
    log.set_level(initial.settings.log_level);
    if let Err(err) = commands::apply_autostart(app.handle(), initial.settings.autostart) {
        tracing::warn!(?err, "initial autostart synchronisation failed");
    }
    let tray_items = tray::create(app, &initial)?;
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    let handle = app.handle().clone();
    let mut updates = runtime.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut last_level = initial.settings.log_level;
        while updates.changed().await.is_ok() {
            let snapshot = updates.borrow_and_update().clone();
            if snapshot.settings.log_level != last_level {
                last_level = snapshot.settings.log_level;
                handle.state::<AppState>().log.set_level(last_level);
            }
            if snapshot.last_error.as_ref().is_some_and(|error| error.code == "version_mismatch") {
                handle.state::<updater::UpdateManager>().request_check();
            }
            tray::update(&handle, &tray_items, &snapshot);
            let _ = handle.emit("status", &snapshot);
            tokio::time::sleep(STATUS_EMIT_INTERVAL).await;
        }
    });
    app.manage(AppState { runtime, log, log_dir });
    app.manage(updater::UpdateManager::new());
    tauri::async_runtime::spawn(updater::run(app.handle().clone()));
    let launched_in_background = std::env::args().any(|arg| arg == BACKGROUND_ARG);
    if !launched_in_background {
        tray::show_main_window(app.handle());
    }
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main_window(app)))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![BACKGROUND_ARG])))
        .setup(setup)
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::update_settings,
            commands::start_pairing,
            commands::cancel_pairing,
            commands::join_pairing,
            commands::approve_pairing,
            commands::remove_peer,
            commands::set_manual_address,
            commands::reconnect_now,
            commands::resync_clipboard,
            commands::release_control,
            commands::request_permissions,
            commands::reset_permissions,
            commands::start_viewer,
            commands::stop_viewer,
            commands::restart_app,
            updater::get_update_status,
            updater::check_for_updates,
            updater::install_update,
            commands::open_permission_settings,
            commands::reset_identity,
            commands::clear_error,
            commands::open_log_folder,
            commands::diagnostics_report,
        ])
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Unify failed to start: {err}");
            std::process::exit(1);
        }
    };
    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                let runtime = state.runtime.clone();
                tauri::async_runtime::block_on(async move {
                    let _ = tokio::time::timeout(Duration::from_secs(3), runtime.shutdown()).await;
                });
            }
        }
    });
}
