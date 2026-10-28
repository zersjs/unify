mod clipboard;
mod display;
mod hook;
mod inject;

use std::sync::Arc;

use unify_core::geometry::Desktop;
use unify_proto::OsKind;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

use crate::{
    CaptureShared, CaptureSink, ChangeSink, ClipboardText, Injector, PermissionKind, PermissionReport, PermissionState, Platform,
    PlatformError, StopHandle,
};

pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> Self {
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
        Self
    }
}

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for WindowsPlatform {
    fn os(&self) -> OsKind {
        OsKind::Windows
    }

    fn default_device_name(&self) -> String {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| String::from("Windows PC"))
    }

    fn permissions(&self) -> PermissionReport {
        PermissionReport {
            accessibility: PermissionState::NotRequired,
            input_monitoring: PermissionState::NotRequired,
            local_network: PermissionState::NotRequired,
        }
    }

    fn request_permissions(&self) {}

    fn open_permission_settings(&self, _kind: PermissionKind) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn detect_iso_keyboard(&self) -> bool {
        false
    }

    fn parks_cursor_at_exit(&self) -> bool {
        false
    }

    fn desktop(&self) -> Result<Desktop, PlatformError> {
        display::desktop()
    }

    fn cursor_position(&self) -> Option<(i32, i32)> {
        let mut point = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut point) } == 0 {
            return None;
        }
        Some((point.x, point.y))
    }

    fn warp_cursor(&self, x: i32, y: i32) -> Result<(), PlatformError> {
        if unsafe { SetCursorPos(x, y) } == 0 {
            return Err(PlatformError::Os(format!("SetCursorPos failed: {}", std::io::Error::last_os_error())));
        }
        Ok(())
    }

    fn start_capture(&self, shared: Arc<CaptureShared>, sink: CaptureSink) -> Result<Box<dyn StopHandle>, PlatformError> {
        Ok(Box::new(hook::HookThread::start(shared, sink)?))
    }

    fn injector(&self, _shared: Arc<CaptureShared>) -> Result<Arc<dyn Injector>, PlatformError> {
        Ok(Arc::new(inject::WindowsInjector))
    }

    fn start_clipboard_watch(&self, sink: ChangeSink) -> Result<Box<dyn StopHandle>, PlatformError> {
        Ok(Box::new(clipboard::ClipboardWatcher::start(sink)?))
    }

    fn read_clipboard(&self) -> Result<Option<ClipboardText>, PlatformError> {
        clipboard::read_text()
    }

    fn write_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        clipboard::write_text(text)
    }
}
