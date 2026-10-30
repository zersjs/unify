use std::ffi::c_void;

use core_graphics::geometry::CGPoint;

pub type CGEventTapProxy = *const c_void;
pub type RawEvent = *mut c_void;
pub type MachPortRef = *mut c_void;
pub type TapCallback = unsafe extern "C" fn(CGEventTapProxy, u32, RawEvent, *mut c_void) -> RawEvent;

pub const SESSION_EVENT_TAP: u32 = 1;
pub const HEAD_INSERT_EVENT_TAP: u32 = 0;
pub const TAP_OPTION_DEFAULT: u32 = 0;

pub const EVENT_LEFT_MOUSE_DOWN: u32 = 1;
pub const EVENT_LEFT_MOUSE_UP: u32 = 2;
pub const EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
pub const EVENT_RIGHT_MOUSE_UP: u32 = 4;
pub const EVENT_MOUSE_MOVED: u32 = 5;
pub const EVENT_LEFT_MOUSE_DRAGGED: u32 = 6;
pub const EVENT_RIGHT_MOUSE_DRAGGED: u32 = 7;
pub const EVENT_KEY_DOWN: u32 = 10;
pub const EVENT_KEY_UP: u32 = 11;
pub const EVENT_FLAGS_CHANGED: u32 = 12;
pub const EVENT_SCROLL_WHEEL: u32 = 22;
pub const EVENT_OTHER_MOUSE_DOWN: u32 = 25;
pub const EVENT_OTHER_MOUSE_UP: u32 = 26;
pub const EVENT_OTHER_MOUSE_DRAGGED: u32 = 27;
pub const EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
pub const EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

pub const FIELD_MOUSE_BUTTON_NUMBER: u32 = 3;
pub const FIELD_MOUSE_DELTA_X: u32 = 4;
pub const FIELD_MOUSE_DELTA_Y: u32 = 5;
pub const FIELD_KEYBOARD_KEYCODE: u32 = 9;
pub const FIELD_SCROLL_POINT_DELTA_AXIS_1: u32 = 96;
pub const FIELD_SCROLL_POINT_DELTA_AXIS_2: u32 = 97;
pub const FIELD_EVENT_SOURCE_USER_DATA: u32 = 42;

pub const KEYBOARD_LAYOUT_ISO: u32 = u32::from_be_bytes(*b"ISO ");

pub fn event_mask() -> u64 {
    [
        EVENT_LEFT_MOUSE_DOWN,
        EVENT_LEFT_MOUSE_UP,
        EVENT_RIGHT_MOUSE_DOWN,
        EVENT_RIGHT_MOUSE_UP,
        EVENT_MOUSE_MOVED,
        EVENT_LEFT_MOUSE_DRAGGED,
        EVENT_RIGHT_MOUSE_DRAGGED,
        EVENT_KEY_DOWN,
        EVENT_KEY_UP,
        EVENT_FLAGS_CHANGED,
        EVENT_SCROLL_WHEEL,
        EVENT_OTHER_MOUSE_DOWN,
        EVENT_OTHER_MOUSE_UP,
        EVENT_OTHER_MOUSE_DRAGGED,
    ]
    .iter()
    .fold(0u64, |mask, kind| mask | (1u64 << kind))
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: TapCallback,
        user_info: *mut c_void,
    ) -> MachPortRef;
    pub fn CGEventTapEnable(tap: MachPortRef, enable: bool);
    pub fn CGEventGetIntegerValueField(event: RawEvent, field: u32) -> i64;
    pub fn CGEventGetLocation(event: RawEvent) -> CGPoint;
    pub fn CGEventGetFlags(event: RawEvent) -> u64;
    pub fn CGAssociateMouseAndMouseCursorPosition(connected: u32) -> i32;
    pub fn CGWarpMouseCursorPosition(point: CGPoint) -> i32;
    pub fn CGPreflightListenEventAccess() -> bool;
    pub fn CGRequestListenEventAccess() -> bool;
    pub fn CGPreflightPostEventAccess() -> bool;
    pub fn CGRequestPostEventAccess() -> bool;
    pub fn CGRequestScreenCaptureAccess() -> bool;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn LMGetKbdType() -> u8;
    pub fn KBGetLayoutType(keyboard_type: i16) -> u32;
}
