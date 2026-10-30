use std::sync::{Arc, Mutex};

use unify_core::geometry::{Desktop, Rect};
use unify_proto::OsKind;

use crate::{
    CaptureShared, CaptureSink, ChangeSink, ClipboardText, EncodedFrame, FrameSink, InjectEvent, Injector, PermissionKind, PermissionReport,
    PermissionState, Platform, PlatformError, ScreenCapture, ScreenSettings, StopHandle, Viewer, ViewerEvent, ViewerSink,
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
    injection_denied: bool,
    capture_denied: bool,
    screen_denied: bool,
    frame_sink: Option<FrameSink>,
    keyframe_requests: u32,
    viewer_sink: Option<ViewerSink>,
    viewer_frames: Vec<EncodedFrame>,
    viewer_size: Option<(u32, u32)>,
    mode_restored: u32,
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

    pub fn set_injection_denied(&self, denied: bool) {
        self.with(|state| state.injection_denied = denied);
    }

    pub fn set_capture_denied(&self, denied: bool) {
        self.with(|state| state.capture_denied = denied);
    }

    pub fn set_screen_denied(&self, denied: bool) {
        self.with(|state| state.screen_denied = denied);
    }

    pub fn emit_screen_frame(&self, frame: EncodedFrame) -> bool {
        match self.with(|state| state.frame_sink.clone()) {
            Some(sink) => {
                sink(frame);
                true
            }
            None => false,
        }
    }

    pub fn emit_viewer_event(&self, event: ViewerEvent) {
        if let Some(sink) = self.with(|state| state.viewer_sink.clone()) {
            sink(event);
        }
    }

    pub fn viewer_frames(&self) -> Vec<EncodedFrame> {
        self.with(|state| state.viewer_frames.clone())
    }

    pub fn viewer_size(&self) -> Option<(u32, u32)> {
        self.with(|state| state.viewer_size)
    }

    pub fn keyframe_requests(&self) -> u32 {
        self.with(|state| state.keyframe_requests)
    }

    pub fn mode_restored(&self) -> u32 {
        self.with(|state| state.mode_restored)
    }

    pub fn warps(&self) -> Vec<(i32, i32)> {
        self.with(|state| state.warps.clone())
    }
}

struct FakeScreen {
    state: Arc<Mutex<FakeState>>,
}

impl StopHandle for FakeScreen {
    fn stop(self: Box<Self>) {
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.frame_sink = None;
    }
}

impl ScreenCapture for FakeScreen {
    fn request_keyframe(&self) {
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.keyframe_requests += 1;
    }
}

struct FakeViewer {
    state: Arc<Mutex<FakeState>>,
}

impl Viewer for FakeViewer {
    fn submit(&self, frame: EncodedFrame) -> bool {
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.viewer_frames.push(frame);
        true
    }

    fn stop(self: Box<Self>) {
        let mut guard = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.viewer_sink = None;
        guard.viewer_size = None;
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

    fn reset_permissions(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

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
        if self.with(|state| state.capture_denied) {
            return Err(PlatformError::PermissionDenied("accessibility"));
        }
        self.with(|state| {
            state.capture_sink = Some(sink);
            state.capture_shared = Some(shared);
        });
        Ok(Box::new(FakeStop))
    }

    fn injector(&self, _shared: Arc<CaptureShared>) -> Result<Arc<dyn Injector>, PlatformError> {
        if self.with(|state| state.injection_denied) {
            return Err(PlatformError::PermissionDenied("accessibility"));
        }
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

    fn start_screen_capture(&self, _settings: ScreenSettings, sink: FrameSink) -> Result<Box<dyn ScreenCapture>, PlatformError> {
        if self.with(|state| state.screen_denied) {
            return Err(PlatformError::PermissionDenied("screen_recording"));
        }
        self.with(|state| state.frame_sink = Some(sink));
        Ok(Box::new(FakeScreen { state: self.state.clone() }))
    }

    fn start_viewer(&self, _title: &str, stream: (u32, u32), sink: ViewerSink) -> Result<Box<dyn Viewer>, PlatformError> {
        self.with(|state| {
            state.viewer_sink = Some(sink);
            state.viewer_size = Some(stream);
        });
        Ok(Box::new(FakeViewer { state: self.state.clone() }))
    }

    fn match_display_mode(&self, width: u32, height: u32) -> Result<(u32, u32), PlatformError> {
        Ok((width, height))
    }

    fn restore_display_mode(&self) {
        self.with(|state| state.mode_restored += 1);
    }
}
