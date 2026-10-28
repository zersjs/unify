use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

use core_foundation::base::TCFType;
use core_foundation::mach_port::{CFMachPort, CFMachPortRef};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::geometry::CGPoint;
use unify_core::capture::{ButtonAction, CaptureFilter, KeyAction};
use unify_proto::keymap::{mac_keycode_to_usage, usage};
use unify_proto::{InputEvent, MouseButton};

use super::ffi::*;
use crate::mac_logic::{button_from_number, flags_change, wheel_from_mac, FlagsChange};
use crate::{CaptureEvent, CaptureShared, CaptureSink, PlatformError, StopHandle, INJECTION_MARKER};

struct TapState {
    shared: Arc<CaptureShared>,
    sink: CaptureSink,
    filter: CaptureFilter,
    last_generation: u64,
    port: MachPortRef,
}

fn warp(x: i32, y: i32) {
    unsafe {
        CGWarpMouseCursorPosition(CGPoint::new(f64::from(x), f64::from(y)));
        CGAssociateMouseAndMouseCursorPosition(1);
    }
}

impl TapState {
    fn emit(&self, event: CaptureEvent) {
        (self.sink)(event);
    }

    fn key(&mut self, code: u16, down: bool, remote: bool) -> bool {
        match self.filter.on_key(code, down, remote) {
            KeyAction::PassThrough => false,
            KeyAction::Swallow => true,
            KeyAction::Forward => {
                self.emit(CaptureEvent::Input(InputEvent::Key { usage: code, down }));
                true
            }
            KeyAction::Hotkey => {
                self.emit(CaptureEvent::Hotkey);
                true
            }
        }
    }

    fn button(&mut self, button: MouseButton, down: bool, remote: bool) -> bool {
        let action = self.filter.on_button(button, down, remote);
        self.shared.set_local_buttons_down(self.filter.local_buttons_down());
        match action {
            ButtonAction::PassThrough => false,
            ButtonAction::Forward => {
                self.emit(CaptureEvent::Input(InputEvent::MouseButton { button, down }));
                true
            }
        }
    }

    unsafe fn handle(&mut self, kind: u32, event: RawEvent) -> bool {
        let remote = self.shared.is_remote();
        self.filter.set_hotkey_enabled(self.shared.hotkey_enabled());
        match kind {
            EVENT_KEY_DOWN | EVENT_KEY_UP => {
                let keycode = CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16;
                match mac_keycode_to_usage(keycode, self.shared.iso_keyboard()) {
                    Some(code) => self.key(code, kind == EVENT_KEY_DOWN, remote),
                    None => remote,
                }
            }
            EVENT_FLAGS_CHANGED => {
                let keycode = CGEventGetIntegerValueField(event, FIELD_KEYBOARD_KEYCODE) as u16;
                match flags_change(keycode, CGEventGetFlags(event)) {
                    FlagsChange::Modifier { usage, down } => self.key(usage, down, remote),
                    FlagsChange::CapsLockToggle => {
                        let swallow_down = self.key(usage::CAPS_LOCK, true, remote);
                        let swallow_up = self.key(usage::CAPS_LOCK, false, remote);
                        swallow_down || swallow_up
                    }
                    FlagsChange::Ignored => remote,
                }
            }
            EVENT_MOUSE_MOVED | EVENT_LEFT_MOUSE_DRAGGED | EVENT_RIGHT_MOUSE_DRAGGED | EVENT_OTHER_MOUSE_DRAGGED => {
                if !remote {
                    let location = CGEventGetLocation(event);
                    self.emit(CaptureEvent::LocalMotion { x: location.x.floor() as i32, y: location.y.floor() as i32 });
                    return false;
                }
                let generation = self.shared.generation();
                let (park_x, park_y) = self.shared.park();
                if generation != self.last_generation {
                    self.last_generation = generation;
                    warp(park_x, park_y);
                    return true;
                }
                let dx = CGEventGetIntegerValueField(event, FIELD_MOUSE_DELTA_X) as i32;
                let dy = CGEventGetIntegerValueField(event, FIELD_MOUSE_DELTA_Y) as i32;
                if dx != 0 || dy != 0 {
                    self.emit(CaptureEvent::Input(InputEvent::MouseMove { dx, dy }));
                }
                warp(park_x, park_y);
                true
            }
            EVENT_LEFT_MOUSE_DOWN | EVENT_LEFT_MOUSE_UP => self.button(MouseButton::Left, kind == EVENT_LEFT_MOUSE_DOWN, remote),
            EVENT_RIGHT_MOUSE_DOWN | EVENT_RIGHT_MOUSE_UP => self.button(MouseButton::Right, kind == EVENT_RIGHT_MOUSE_DOWN, remote),
            EVENT_OTHER_MOUSE_DOWN | EVENT_OTHER_MOUSE_UP => {
                match button_from_number(CGEventGetIntegerValueField(event, FIELD_MOUSE_BUTTON_NUMBER)) {
                    Some(button) => self.button(button, kind == EVENT_OTHER_MOUSE_DOWN, remote),
                    None => remote,
                }
            }
            EVENT_SCROLL_WHEEL => {
                if !remote {
                    return false;
                }
                let (dx, dy) = wheel_from_mac(
                    CGEventGetIntegerValueField(event, FIELD_SCROLL_POINT_DELTA_AXIS_1),
                    CGEventGetIntegerValueField(event, FIELD_SCROLL_POINT_DELTA_AXIS_2),
                );
                if dx != 0 || dy != 0 {
                    self.emit(CaptureEvent::Input(InputEvent::Wheel { dx, dy }));
                }
                true
            }
            _ => false,
        }
    }
}

unsafe extern "C" fn tap_callback(_proxy: CGEventTapProxy, kind: u32, event: RawEvent, user_info: *mut c_void) -> RawEvent {
    let Some(state) = (user_info as *mut TapState).as_mut() else {
        return event;
    };
    if kind == EVENT_TAP_DISABLED_BY_TIMEOUT || kind == EVENT_TAP_DISABLED_BY_USER_INPUT {
        if !state.port.is_null() {
            CGEventTapEnable(state.port, true);
        }
        state.emit(CaptureEvent::TapReenabled);
        return event;
    }
    if event.is_null() {
        return event;
    }
    if CGEventGetIntegerValueField(event, FIELD_EVENT_SOURCE_USER_DATA) == INJECTION_MARKER as i64 {
        return event;
    }
    let swallow = catch_unwind(AssertUnwindSafe(|| state.handle(kind, event))).unwrap_or(false);
    if swallow {
        std::ptr::null_mut()
    } else {
        event
    }
}

struct SendableRunLoop(CFRunLoop);

pub struct EventTapThread {
    run_loop: CFRunLoop,
    join: Option<JoinHandle<()>>,
}

impl EventTapThread {
    pub fn start(shared: Arc<CaptureShared>, sink: CaptureSink) -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<SendableRunLoop, PlatformError>>();
        let join = std::thread::Builder::new()
            .name("unify-event-tap".into())
            .spawn(move || {
                let hotkey = shared.hotkey_enabled();
                let state = Box::into_raw(Box::new(TapState {
                    shared,
                    sink,
                    filter: CaptureFilter::new(hotkey),
                    last_generation: 0,
                    port: std::ptr::null_mut(),
                }));
                let port = unsafe {
                    CGEventTapCreate(
                        SESSION_EVENT_TAP,
                        HEAD_INSERT_EVENT_TAP,
                        TAP_OPTION_DEFAULT,
                        event_mask(),
                        tap_callback,
                        state as *mut c_void,
                    )
                };
                if port.is_null() {
                    drop(unsafe { Box::from_raw(state) });
                    let _ = ready_tx.send(Err(PlatformError::PermissionDenied("accessibility")));
                    return;
                }
                unsafe {
                    (*state).port = port;
                }
                let mach_port = unsafe { CFMachPort::wrap_under_create_rule(port as CFMachPortRef) };
                let source = match mach_port.create_runloop_source(0) {
                    Ok(source) => source,
                    Err(()) => {
                        drop(unsafe { Box::from_raw(state) });
                        let _ = ready_tx.send(Err(PlatformError::Os(String::from("CFMachPortCreateRunLoopSource failed"))));
                        return;
                    }
                };
                let run_loop = CFRunLoop::get_current();
                unsafe {
                    run_loop.add_source(&source, kCFRunLoopCommonModes);
                    CGEventTapEnable(port, true);
                }
                let _ = ready_tx.send(Ok(SendableRunLoop(run_loop.clone())));
                CFRunLoop::run_current();
                unsafe {
                    CGEventTapEnable(port, false);
                }
                run_loop.remove_source(&source, unsafe { kCFRunLoopCommonModes });
                drop(source);
                drop(mach_port);
                drop(unsafe { Box::from_raw(state) });
            })
            .map_err(|err| PlatformError::Os(err.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(SendableRunLoop(run_loop))) => Ok(Self { run_loop, join: Some(join) }),
            Ok(Err(err)) => {
                let _ = join.join();
                Err(err)
            }
            Err(_) => Err(PlatformError::Os(String::from("event tap thread exited during startup"))),
        }
    }
}

impl StopHandle for EventTapThread {
    fn stop(mut self: Box<Self>) {
        let Some(join) = self.join.take() else {
            return;
        };
        for _ in 0..200 {
            self.run_loop.stop();
            if join.is_finished() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let _ = join.join();
    }
}
