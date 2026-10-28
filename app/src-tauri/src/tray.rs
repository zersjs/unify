use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, Wry};
use unify_core::config::{SettingsPatch, UiLanguage};
use unify_runtime::{ConnectionState, ControlState, StatusSnapshot};

use crate::AppState;

const TRAY_ID: &str = "unify-tray";

struct Labels {
    show: &'static str,
    input: &'static str,
    clipboard: &'static str,
    resync: &'static str,
    release: &'static str,
    quit: &'static str,
    not_paired: &'static str,
    waiting: &'static str,
    connecting: &'static str,
    reconnecting: &'static str,
    connected: &'static str,
}

const ENGLISH: Labels = Labels {
    show: "Show Unify",
    input: "Keyboard & mouse sharing",
    clipboard: "Clipboard sync",
    resync: "Sync clipboard now",
    release: "Take back control",
    quit: "Quit Unify",
    not_paired: "Pairing required",
    waiting: "Waiting for connection",
    connecting: "Connecting",
    reconnecting: "Reconnecting",
    connected: "Connected",
};

const TURKISH: Labels = Labels {
    show: "Unify'ı göster",
    input: "Klavye ve fare paylaşımı",
    clipboard: "Pano eşitleme",
    resync: "Panoyu şimdi eşitle",
    release: "Kontrolü geri al",
    quit: "Unify'dan çık",
    not_paired: "Eşleştirme gerekli",
    waiting: "Bağlantı bekleniyor",
    connecting: "Bağlanıyor",
    reconnecting: "Yeniden bağlanılıyor",
    connected: "Bağlandı",
};

fn labels(language: UiLanguage) -> &'static Labels {
    match language {
        UiLanguage::En => &ENGLISH,
        UiLanguage::Tr => &TURKISH,
    }
}

pub struct TrayItems {
    status: MenuItem<Wry>,
    show: MenuItem<Wry>,
    input: CheckMenuItem<Wry>,
    clipboard: CheckMenuItem<Wry>,
    resync: MenuItem<Wry>,
    release: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

fn connection_text(status: &StatusSnapshot) -> String {
    let labels = labels(status.settings.language);
    let state = match status.connection.state {
        ConnectionState::NotPaired => labels.not_paired,
        ConnectionState::Waiting => labels.waiting,
        ConnectionState::Connecting => labels.connecting,
        ConnectionState::Reconnecting => labels.reconnecting,
        ConnectionState::Connected => labels.connected,
    };
    match (&status.peer, status.connection.state) {
        (Some(peer), ConnectionState::Connected) => format!("{state}: {}", peer.name),
        _ => state.to_owned(),
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn create(app: &App, status: &StatusSnapshot) -> tauri::Result<TrayItems> {
    let text = labels(status.settings.language);
    let status_item = MenuItem::with_id(app, "status", connection_text(status), false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", text.show, true, None::<&str>)?;
    let input = CheckMenuItem::with_id(app, "input", text.input, true, status.settings.input_sharing, None::<&str>)?;
    let clipboard = CheckMenuItem::with_id(app, "clipboard", text.clipboard, true, status.settings.clipboard_sync, None::<&str>)?;
    let resync = MenuItem::with_id(app, "resync", text.resync, true, None::<&str>)?;
    let release = MenuItem::with_id(app, "release", text.release, status.control != ControlState::Local, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", text.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status_item,
            &PredefinedMenuItem::separator(app)?,
            &show,
            &PredefinedMenuItem::separator(app)?,
            &input,
            &clipboard,
            &resync,
            &release,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Unify")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            let runtime = state.runtime.clone();
            match event.id().as_ref() {
                "show" => show_main_window(app),
                "input" => {
                    let enabled = !runtime.status().settings.input_sharing;
                    tauri::async_runtime::spawn(async move {
                        let _ = runtime.update_settings(SettingsPatch { input_sharing: Some(enabled), ..Default::default() }).await;
                    });
                }
                "clipboard" => {
                    let enabled = !runtime.status().settings.clipboard_sync;
                    tauri::async_runtime::spawn(async move {
                        let _ = runtime.update_settings(SettingsPatch { clipboard_sync: Some(enabled), ..Default::default() }).await;
                    });
                }
                "resync" => {
                    let _ = runtime.resync_clipboard();
                }
                "release" => {
                    let _ = runtime.release_control();
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if cfg!(target_os = "macos") {
                return;
            }
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(TrayItems { status: status_item, show, input, clipboard, resync, release, quit })
}

pub fn update(app: &AppHandle, items: &TrayItems, status: &StatusSnapshot) {
    let text = labels(status.settings.language);
    let connection = connection_text(status);
    let _ = items.status.set_text(&connection);
    let _ = items.show.set_text(text.show);
    let _ = items.input.set_text(text.input);
    let _ = items.clipboard.set_text(text.clipboard);
    let _ = items.resync.set_text(text.resync);
    let _ = items.release.set_text(text.release);
    let _ = items.quit.set_text(text.quit);
    let _ = items.input.set_checked(status.settings.input_sharing);
    let _ = items.clipboard.set_checked(status.settings.clipboard_sync);
    let _ = items.release.set_enabled(status.control != ControlState::Local);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(format!("Unify — {connection}")));
    }
}
