mod clipboard;
mod ffi;
mod inject;
mod tap;

use std::process::Command;
use std::sync::{Arc, Mutex};

use core_graphics::display::CGDisplay;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use unify_core::geometry::{Desktop, Rect};
use unify_proto::OsKind;

use crate::{
    CaptureShared, CaptureSink, ChangeSink, ClipboardText, Injector, PermissionKind, PermissionReport, PermissionState, Platform,
    PlatformError, StopHandle,
};

pub struct MacPlatform {
    injector: Mutex<Option<Arc<inject::MacInjector>>>,
}

impl MacPlatform {
    pub fn new() -> Self {
        Self { injector: Mutex::new(None) }
    }
}

impl Default for MacPlatform {
    fn default() -> Self {
        Self::new()
    }
}

fn state(granted: bool) -> PermissionState {
    if granted {
        PermissionState::Granted
    } else {
        PermissionState::Denied
    }
}

impl Platform for MacPlatform {
    fn os(&self) -> OsKind {
        OsKind::MacOs
    }

    fn default_device_name(&self) -> String {
        Command::new("/usr/sbin/scutil")
            .args(["--get", "ComputerName"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| String::from("Mac"))
    }

    fn permissions(&self) -> PermissionReport {
        let (accessibility, listen) = unsafe { (ffi::AXIsProcessTrusted(), ffi::CGPreflightListenEventAccess()) };
        PermissionReport {
            accessibility: state(accessibility),
            input_monitoring: state(listen),
            local_network: PermissionState::Unknown,
        }
    }

    fn request_permissions(&self) {
        unsafe {
            if !ffi::CGPreflightListenEventAccess() {
                ffi::CGRequestListenEventAccess();
            }
            if !ffi::CGPreflightPostEventAccess() {
                ffi::CGRequestPostEventAccess();
            }
        }
    }

    fn open_permission_settings(&self, kind: PermissionKind) -> Result<(), PlatformError> {
        let anchor = match kind {
            PermissionKind::Accessibility => "Privacy_Accessibility",
            PermissionKind::InputMonitoring => "Privacy_ListenEvent",
            PermissionKind::LocalNetwork => "Privacy_LocalNetwork",
        };
        let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
        Command::new("/usr/bin/open").arg(url).spawn().map(|_| ()).map_err(|err| PlatformError::Os(err.to_string()))
    }

    fn detect_iso_keyboard(&self) -> bool {
        unsafe { ffi::KBGetLayoutType(i16::from(ffi::LMGetKbdType())) == ffi::KEYBOARD_LAYOUT_ISO }
    }

    fn parks_cursor_at_exit(&self) -> bool {
        true
    }

    fn desktop(&self) -> Result<Desktop, PlatformError> {
        let ids = CGDisplay::active_displays().map_err(|code| PlatformError::Os(format!("CGGetActiveDisplayList failed: {code}")))?;
        let rects = ids
            .into_iter()
            .map(|id| {
                let bounds = CGDisplay::new(id).bounds();
                Rect::new(
                    bounds.origin.x.floor() as i32,
                    bounds.origin.y.floor() as i32,
                    bounds.size.width.round() as i32,
                    bounds.size.height.round() as i32,
                )
            })
            .collect();
        Desktop::new(rects).ok_or_else(|| PlatformError::Os(String::from("no active displays")))
    }

    fn cursor_position(&self) -> Option<(i32, i32)> {
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
        let location = CGEvent::new(source).ok()?.location();
        Some((location.x.floor() as i32, location.y.floor() as i32))
    }

    fn warp_cursor(&self, x: i32, y: i32) -> Result<(), PlatformError> {
        let result = unsafe { ffi::CGWarpMouseCursorPosition(CGPoint::new(f64::from(x), f64::from(y))) };
        unsafe {
            ffi::CGAssociateMouseAndMouseCursorPosition(1);
        }
        if result != 0 {
            return Err(PlatformError::Os(format!("CGWarpMouseCursorPosition failed: {result}")));
        }
        Ok(())
    }

    fn start_capture(&self, shared: Arc<CaptureShared>, sink: CaptureSink) -> Result<Box<dyn StopHandle>, PlatformError> {
        if !unsafe { ffi::CGPreflightListenEventAccess() } {
            return Err(PlatformError::PermissionDenied("input_monitoring"));
        }
        Ok(Box::new(tap::EventTapThread::start(shared, sink)?))
    }

    fn injector(&self, shared: Arc<CaptureShared>) -> Result<Arc<dyn Injector>, PlatformError> {
        if !unsafe { ffi::CGPreflightPostEventAccess() } {
            return Err(PlatformError::PermissionDenied("accessibility"));
        }
        let mut guard = self.injector.lock().map_err(|_| PlatformError::NotRunning)?;
        if let Some(existing) = guard.as_ref() {
            return Ok(existing.clone());
        }
        let created = Arc::new(inject::MacInjector::start(shared)?);
        *guard = Some(created.clone());
        Ok(created)
    }

    fn start_clipboard_watch(&self, sink: ChangeSink) -> Result<Box<dyn StopHandle>, PlatformError> {
        Ok(Box::new(clipboard::ClipboardPoller::start(sink)?))
    }

    fn read_clipboard(&self) -> Result<Option<ClipboardText>, PlatformError> {
        clipboard::read_text()
    }

    fn write_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        clipboard::write_text(text)
    }
}
