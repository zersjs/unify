mod clipboard;
mod ffi;
mod inject;
mod screen;
mod tap;

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use core_graphics::display::CGDisplay;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use unify_core::geometry::{Desktop, Rect};
use unify_proto::OsKind;

use crate::{
    CaptureShared, CaptureSink, ChangeSink, ClipboardText, FrameSink, Injector, PermissionKind, PermissionReport, PermissionState, Platform,
    PlatformError, ScreenCapture, ScreenSettings, StopHandle, Viewer, ViewerSink,
};

const BUNDLE_IDENTIFIER: &str = "dev.unify.desktop";
const TCC_SERVICES: [&str; 3] = ["Accessibility", "ListenEvent", "ScreenCapture"];

pub struct MacPlatform {
    injector: Mutex<Option<Arc<inject::MacInjector>>>,
    tap_active: AtomicBool,
}

impl MacPlatform {
    pub fn new() -> Self {
        Self { injector: Mutex::new(None), tap_active: AtomicBool::new(false) }
    }
}

impl MacPlatform {
    fn can_post_events(&self) -> bool {
        unsafe { ffi::AXIsProcessTrusted() || ffi::CGPreflightPostEventAccess() }
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
        let tap_active = self.tap_active.load(Ordering::Relaxed);
        let listen = unsafe { ffi::CGPreflightListenEventAccess() };
        PermissionReport {
            accessibility: state(tap_active || self.can_post_events()),
            input_monitoring: state(tap_active || listen),
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

    fn reset_permissions(&self) -> Result<(), PlatformError> {
        for service in TCC_SERVICES {
            let status = Command::new("/usr/bin/tccutil")
                .args(["reset", service, BUNDLE_IDENTIFIER])
                .status()
                .map_err(|err| PlatformError::Os(err.to_string()))?;
            if !status.success() {
                return Err(PlatformError::Os(format!("tccutil reset {service} failed: {status}")));
            }
        }
        self.request_permissions();
        Ok(())
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
        let thread = tap::EventTapThread::start(shared, sink)?;
        self.tap_active.store(true, Ordering::Relaxed);
        Ok(Box::new(thread))
    }

    fn injector(&self, shared: Arc<CaptureShared>) -> Result<Arc<dyn Injector>, PlatformError> {
        if !self.tap_active.load(Ordering::Relaxed) && !self.can_post_events() {
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

    fn start_screen_capture(&self, settings: ScreenSettings, sink: FrameSink) -> Result<Box<dyn ScreenCapture>, PlatformError> {
        Ok(Box::new(screen::MacScreenCapture::start(settings, sink)?))
    }

    fn start_viewer(&self, _title: &str, _stream: (u32, u32), _sink: ViewerSink) -> Result<Box<dyn Viewer>, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn match_display_mode(&self, width: u32, height: u32) -> Result<(u32, u32), PlatformError> {
        screen::match_display_mode(width, height)
    }

    fn restore_display_mode(&self) {
        screen::restore_display_mode();
    }
}
