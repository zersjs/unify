use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Notify;
use unify_runtime::{ControlState, StreamState};

use crate::AppState;

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
const LOOP_INTERVAL: Duration = Duration::from_secs(60);
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);
const FORCED_CHECK_SPACING: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available { version: String },
    Downloading { version: String, percent: u8 },
    Installing { version: String },
    Failed { code: &'static str },
}

pub struct UpdateManager {
    status: Mutex<UpdateStatus>,
    pending: Mutex<Option<Update>>,
    last_check: Mutex<Option<Instant>>,
    forced: Notify,
}

impl UpdateManager {
    pub fn new() -> Self {
        Self { status: Mutex::new(UpdateStatus::Idle), pending: Mutex::new(None), last_check: Mutex::new(None), forced: Notify::new() }
    }

    fn status(&self) -> UpdateStatus {
        self.status.lock().map(|status| status.clone()).unwrap_or(UpdateStatus::Idle)
    }

    pub fn request_check(&self) {
        self.forced.notify_one();
    }

    fn checked_within(&self, spacing: Duration) -> bool {
        self.last_check.lock().ok().and_then(|last| *last).is_some_and(|last| last.elapsed() < spacing)
    }

    fn has_pending(&self) -> bool {
        self.pending.lock().is_ok_and(|pending| pending.is_some())
    }

    fn take_pending(&self) -> Option<Update> {
        self.pending.lock().ok().and_then(|mut pending| pending.take())
    }
}

fn publish(app: &AppHandle, status: UpdateStatus) {
    let manager = app.state::<UpdateManager>();
    if let Ok(mut current) = manager.status.lock() {
        *current = status.clone();
    }
    let _ = app.emit("update", &status);
}

async fn check(app: &AppHandle) {
    let manager = app.state::<UpdateManager>();
    if let Ok(mut last) = manager.last_check.lock() {
        *last = Some(Instant::now());
    }
    publish(app, UpdateStatus::Checking);
    let result = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(err) => Err(err),
    };
    match result {
        Ok(Some(update)) => {
            let version = update.version.clone();
            if let Ok(mut pending) = manager.pending.lock() {
                *pending = Some(update);
            }
            publish(app, UpdateStatus::Available { version });
        }
        Ok(None) => publish(app, UpdateStatus::UpToDate),
        Err(err) => {
            tracing::warn!(error = %err, "update check failed");
            publish(app, UpdateStatus::Failed { code: "update_check_failed" });
        }
    }
}

async fn install(app: &AppHandle) {
    let Some(update) = app.state::<UpdateManager>().take_pending() else {
        return;
    };
    let version = update.version.clone();
    let progress_app = app.clone();
    let progress_version = version.clone();
    let mut received = 0u64;
    let mut last_percent = 0u8;
    publish(app, UpdateStatus::Downloading { version: version.clone(), percent: 0 });
    let downloaded = update
        .download(
            move |chunk, total| {
                received += chunk as u64;
                let percent = total.filter(|total| *total > 0).map_or(0, |total| (received * 100 / total).min(100) as u8);
                if percent != last_percent {
                    last_percent = percent;
                    publish(&progress_app, UpdateStatus::Downloading { version: progress_version.clone(), percent });
                }
            },
            || {},
        )
        .await;
    let bytes = match downloaded {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(error = %err, "update download failed");
            publish(app, UpdateStatus::Failed { code: "update_download_failed" });
            return;
        }
    };
    publish(app, UpdateStatus::Installing { version });
    app.state::<AppState>().runtime.shutdown().await;
    if let Err(err) = update.install(bytes) {
        tracing::error!(error = %err, "update install failed");
    }
    app.restart();
}

fn is_idle(app: &AppHandle) -> bool {
    let status = app.state::<AppState>().runtime.status();
    status.control == ControlState::Local && status.stream.state == StreamState::Idle
}

pub async fn run(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    tokio::time::sleep(FIRST_CHECK_DELAY).await;
    loop {
        let manager = app.state::<UpdateManager>();
        if !manager.has_pending() && !manager.checked_within(CHECK_INTERVAL) {
            check(&app).await;
        }
        let auto_update = app.state::<AppState>().runtime.status().settings.auto_update;
        if auto_update && manager.has_pending() && is_idle(&app) {
            install(&app).await;
        }
        tokio::select! {
            _ = tokio::time::sleep(LOOP_INTERVAL) => {}
            _ = manager.forced.notified() => {
                if !manager.checked_within(FORCED_CHECK_SPACING) && !manager.has_pending() {
                    check(&app).await;
                }
            }
        }
    }
}

#[tauri::command]
pub fn get_update_status(manager: State<'_, UpdateManager>) -> UpdateStatus {
    manager.status()
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) {
    if !cfg!(debug_assertions) {
        check(&app).await;
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) {
    install(&app).await;
}
