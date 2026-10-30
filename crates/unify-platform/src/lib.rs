pub mod fake;
pub mod mac_logic;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::Arc;

use thiserror::Error;
use unify_core::geometry::Desktop;
use unify_proto::{InputEvent, MouseButton, OsKind};

pub const INJECTION_MARKER: usize = 0x5546_5949;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionState {
    Granted,
    Denied,
    NotRequired,
    Unknown,
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("{0} permission is missing")]
    PermissionDenied(&'static str),
    #[error("operating system call failed: {0}")]
    Os(String),
    #[error("clipboard is busy")]
    ClipboardBusy,
    #[error("component is not running")]
    NotRunning,
    #[error("not supported on this platform")]
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    Accessibility,
    InputMonitoring,
    LocalNetwork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionReport {
    pub accessibility: PermissionState,
    pub input_monitoring: PermissionState,
    pub local_network: PermissionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardText {
    pub text: String,
    pub concealed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEvent {
    LocalMotion { x: i32, y: i32 },
    Input(InputEvent),
    Hotkey,
    TapReenabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectEvent {
    Key { usage: u16, down: bool },
    MoveTo { x: i32, y: i32, dx: i32, dy: i32 },
    Button { button: MouseButton, down: bool },
    Wheel { dx: i32, dy: i32 },
}

#[derive(Debug)]
pub struct CaptureShared {
    remote: AtomicBool,
    generation: AtomicU64,
    park_x: AtomicI32,
    park_y: AtomicI32,
    hotkey_enabled: AtomicBool,
    iso_keyboard: AtomicBool,
    local_buttons_down: AtomicBool,
}

impl CaptureShared {
    pub fn new(hotkey_enabled: bool, iso_keyboard: bool) -> Arc<Self> {
        Arc::new(Self {
            remote: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            park_x: AtomicI32::new(0),
            park_y: AtomicI32::new(0),
            hotkey_enabled: AtomicBool::new(hotkey_enabled),
            iso_keyboard: AtomicBool::new(iso_keyboard),
            local_buttons_down: AtomicBool::new(false),
        })
    }

    pub fn enter_remote(&self, park: (i32, i32)) {
        self.park_x.store(park.0, Ordering::SeqCst);
        self.park_y.store(park.1, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.remote.store(true, Ordering::SeqCst);
    }

    pub fn leave_remote(&self) {
        self.remote.store(false, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn is_remote(&self) -> bool {
        self.remote.load(Ordering::SeqCst)
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn park(&self) -> (i32, i32) {
        (self.park_x.load(Ordering::SeqCst), self.park_y.load(Ordering::SeqCst))
    }

    pub fn set_hotkey_enabled(&self, enabled: bool) {
        self.hotkey_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn hotkey_enabled(&self) -> bool {
        self.hotkey_enabled.load(Ordering::Relaxed)
    }

    pub fn set_iso_keyboard(&self, iso: bool) {
        self.iso_keyboard.store(iso, Ordering::Relaxed);
    }

    pub fn iso_keyboard(&self) -> bool {
        self.iso_keyboard.load(Ordering::Relaxed)
    }

    pub fn set_local_buttons_down(&self, down: bool) {
        self.local_buttons_down.store(down, Ordering::Relaxed);
    }

    pub fn local_buttons_down(&self) -> bool {
        self.local_buttons_down.load(Ordering::Relaxed)
    }
}

pub type CaptureSink = Arc<dyn Fn(CaptureEvent) + Send + Sync>;
pub type ChangeSink = Arc<dyn Fn() + Send + Sync>;

pub trait StopHandle: Send {
    fn stop(self: Box<Self>);
}

pub trait Injector: Send + Sync {
    fn inject(&self, event: InjectEvent) -> Result<(), PlatformError>;
}

pub trait Platform: Send + Sync + 'static {
    fn os(&self) -> OsKind;
    fn default_device_name(&self) -> String;
    fn permissions(&self) -> PermissionReport;
    fn request_permissions(&self);
    fn reset_permissions(&self) -> Result<(), PlatformError>;
    fn open_permission_settings(&self, kind: PermissionKind) -> Result<(), PlatformError>;
    fn detect_iso_keyboard(&self) -> bool;
    fn parks_cursor_at_exit(&self) -> bool;
    fn desktop(&self) -> Result<Desktop, PlatformError>;
    fn cursor_position(&self) -> Option<(i32, i32)>;
    fn warp_cursor(&self, x: i32, y: i32) -> Result<(), PlatformError>;
    fn start_capture(&self, shared: Arc<CaptureShared>, sink: CaptureSink) -> Result<Box<dyn StopHandle>, PlatformError>;
    fn injector(&self, shared: Arc<CaptureShared>) -> Result<Arc<dyn Injector>, PlatformError>;
    fn start_clipboard_watch(&self, sink: ChangeSink) -> Result<Box<dyn StopHandle>, PlatformError>;
    fn read_clipboard(&self) -> Result<Option<ClipboardText>, PlatformError>;
    fn write_clipboard(&self, text: &str) -> Result<(), PlatformError>;
}

pub fn native() -> Result<Arc<dyn Platform>, PlatformError> {
    #[cfg(windows)]
    {
        Ok(Arc::new(windows::WindowsPlatform::new()))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Arc::new(macos::MacPlatform::new()))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(PlatformError::Unsupported)
    }
}
