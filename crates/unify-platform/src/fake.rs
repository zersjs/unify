use std::sync::{Arc, Mutex};

use unify_core::geometry::{Desktop, Rect};
use unify_proto::OsKind;

use crate::{
    CaptureShared, CaptureSink, ChangeSink, ClipboardText, InjectEvent, Injector, PermissionKind, PermissionReport, PermissionState,
    Platform, PlatformError, StopHandle,
};

#[derive(Default)]
struct FakeState {
    clipboard: Option<ClipboardText>,
    clipboard_sink: Option<ChangeSink>,
    capture_sink: Option<CaptureSink>,
    capture_shared: Option<Arc<CaptureShared>>,
    injected: Vec<InjectEvent>,
    cursor: (i32, i32),
    warps: Vec<(i32, i32)>,
}

#[derive(Clone)]
pub struct FakePlatform {
    os: OsKind,
    name: String,
    desktop: Desktop,
    state: Arc<Mutex<FakeState>>,
}

impl FakePlatform {
    pub fn new(os: OsKind, name: &str, width: i32, height: i32) -> Self {
        let desktop = Desktop::new(vec![Rect::new(0, 0, width, height)]).unwrap_or_else(|| unreachable!("non-empty display list"));
        Self { os, name: name.to_owned(), desktop, state: Arc::new(Mutex::new(FakeState::default())) }
    }

    fn with<R>(&self, f: impl FnOnce(&mut FakeState) -> R) -> R {
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard)
    }

    pub fn user_copies(&self, text: &str) {
        let sink = self.with(|state| {
            state.clipboard = Some(ClipboardText { text: text.to_owned(), concealed: false });
            state.clipboard_sink.clone()
        });
        if let Some(sink) = sink {
            sink();
        }
    }

    pub fn user_copies_concealed(&self, text: &str) {
        let sink = self.with(|state| {
            state.clipboard = Some(ClipboardText { text: text.to_owned(), concealed: true });
            state.clipboard_sink.clone()
        });
        if let Some(sink) = sink {
            sink();
        }
    }

    pub fn clipboard_text(&self) -> Option<String> {
        self.with(|state| state.clipboard.as_ref().map(|c| c.text.clone()))
    }

    pub fn emit_capture(&self, event: crate::CaptureEvent) {
        let sink = self.with(|state| state.capture_sink.clone());
        if let Some(sink) = sink {
            sink(event);
        }
    }

    pub fn capture_shared(&self) -> Option<Arc<CaptureShared>> {
        self.with(|state| state.capture_shared.clone())
    }

    pub fn injected(&self) -> Vec<InjectEvent> {
        self.with(|state| state.injected.clone())
    }

    pub fn clear_injected(&self) {
        self.with(|state| state.injected.clear());
    }

    pub fn warps(&self) -> Vec<(i32, i32)> {
        self.with(|state| state.warps.clone())
    }
}

struct FakeStop;

impl StopHandle for FakeStop {
    fn stop(self: Box<Self>) {}
}

struct FakeInjector {
    state: Arc<Mutex<FakeState>>,
}

impl Injector for FakeInjector {
    fn inject(&self, event: InjectEvent) -> Result<(), PlatformError> {
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let InjectEvent::MoveTo { x, y, .. } = event {
            guard.cursor = (x, y);
        }
        guard.injected.push(event);
        Ok(())
    }
}

impl Platform for FakePlatform {
    fn os(&self) -> OsKind {
        self.os
    }

    fn default_device_name(&self) -> String {
        self.name.clone()
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
        Ok(self.desktop.clone())
    }

    fn cursor_position(&self) -> Option<(i32, i32)> {
        Some(self.with(|state| state.cursor))
    }

    fn warp_cursor(&self, x: i32, y: i32) -> Result<(), PlatformError> {
        self.with(|state| {
            state.cursor = (x, y);
            state.warps.push((x, y));
        });
        Ok(())
    }

    fn start_capture(&self, shared: Arc<CaptureShared>, sink: CaptureSink) -> Result<Box<dyn StopHandle>, PlatformError> {
        self.with(|state| {
            state.capture_sink = Some(sink);
            state.capture_shared = Some(shared);
        });
        Ok(Box::new(FakeStop))
    }

    fn injector(&self, _shared: Arc<CaptureShared>) -> Result<Arc<dyn Injector>, PlatformError> {
        Ok(Arc::new(FakeInjector { state: self.state.clone() }))
    }

    fn start_clipboard_watch(&self, sink: ChangeSink) -> Result<Box<dyn StopHandle>, PlatformError> {
        self.with(|state| state.clipboard_sink = Some(sink));
        Ok(Box::new(FakeStop))
    }

    fn read_clipboard(&self) -> Result<Option<ClipboardText>, PlatformError> {
        Ok(self.with(|state| state.clipboard.clone()))
    }

    fn write_clipboard(&self, text: &str) -> Result<(), PlatformError> {
        let sink = self.with(|state| {
            state.clipboard = Some(ClipboardText { text: text.to_owned(), concealed: false });
            state.clipboard_sink.clone()
        });
        if let Some(sink) = sink {
            sink();
        }
        Ok(())
    }
}
