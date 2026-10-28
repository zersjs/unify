use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

use unify_core::capture::{ButtonAction, CaptureFilter, KeyAction};
use unify_proto::keymap::{usage, windows_scancode_to_usage};
use unify_proto::{InputEvent, MouseButton};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, VK_NUMLOCK, VK_PAUSE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetCursorPos, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP,
};

use crate::{CaptureEvent, CaptureShared, CaptureSink, PlatformError, StopHandle};

const XBUTTON1: u16 = 1;
const ALTGR_FAKE_CONTROL_FLAG: u32 = 0x200;

struct HookState {
    shared: Arc<CaptureShared>,
    sink: CaptureSink,
    filter: CaptureFilter,
    last_generation: u64,
}

thread_local! {
    static HOOK_STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

fn usage_for_key(info: &KBDLLHOOKSTRUCT) -> Option<u16> {
    match info.vkCode as u16 {
        VK_PAUSE => return Some(usage::PAUSE),
        VK_NUMLOCK => return Some(usage::NUM_LOCK),
        _ => {}
    }
    let scan = (info.scanCode & 0xFF) as u16;
    if scan == 0 {
        let mapped = unsafe { MapVirtualKeyW(info.vkCode, MAPVK_VK_TO_VSC_EX) } as u16;
        if mapped == 0 {
            return None;
        }
        let code = if mapped & 0xFF00 != 0 { 0xE000 | (mapped & 0xFF) } else { mapped };
        return windows_scancode_to_usage(code);
    }
    let code = if info.flags & LLKHF_EXTENDED != 0 { 0xE000 | scan } else { scan };
    windows_scancode_to_usage(code)
}

impl HookState {
    fn emit(&self, event: CaptureEvent) {
        (self.sink)(event);
    }

    fn on_key(&mut self, info: &KBDLLHOOKSTRUCT, message: u32) -> bool {
        if info.flags & LLKHF_INJECTED != 0 {
            return false;
        }
        let remote = self.shared.is_remote();
        if info.scanCode & ALTGR_FAKE_CONTROL_FLAG != 0 {
            return remote;
        }
        let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let Some(code) = usage_for_key(info) else {
            return remote;
        };
        self.filter.set_hotkey_enabled(self.shared.hotkey_enabled());
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

    fn on_button(&mut self, button: MouseButton, down: bool, remote: bool) -> bool {
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

    fn on_mouse(&mut self, info: &MSLLHOOKSTRUCT, message: u32) -> bool {
        if info.flags & LLMHF_INJECTED != 0 {
            return false;
        }
        let remote = self.shared.is_remote();
        let high_word = (info.mouseData >> 16) as u16;
        match message {
            WM_MOUSEMOVE => {
                if !remote {
                    self.emit(CaptureEvent::LocalMotion { x: info.pt.x, y: info.pt.y });
                    return false;
                }
                let generation = self.shared.generation();
                let (park_x, park_y) = self.shared.park();
                if generation != self.last_generation {
                    self.last_generation = generation;
                    unsafe {
                        SetCursorPos(park_x, park_y);
                    }
                    return true;
                }
                let dx = info.pt.x - park_x;
                let dy = info.pt.y - park_y;
                if dx != 0 || dy != 0 {
                    self.emit(CaptureEvent::Input(InputEvent::MouseMove { dx, dy }));
                }
                true
            }
            WM_LBUTTONDOWN | WM_LBUTTONUP => self.on_button(MouseButton::Left, message == WM_LBUTTONDOWN, remote),
            WM_RBUTTONDOWN | WM_RBUTTONUP => self.on_button(MouseButton::Right, message == WM_RBUTTONDOWN, remote),
            WM_MBUTTONDOWN | WM_MBUTTONUP => self.on_button(MouseButton::Middle, message == WM_MBUTTONDOWN, remote),
            WM_XBUTTONDOWN | WM_XBUTTONUP => {
                let button = if high_word == XBUTTON1 { MouseButton::Back } else { MouseButton::Forward };
                self.on_button(button, message == WM_XBUTTONDOWN, remote)
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                if !remote {
                    return false;
                }
                let delta = i32::from(high_word as i16);
                let event = if message == WM_MOUSEWHEEL {
                    InputEvent::Wheel { dx: 0, dy: delta }
                } else {
                    InputEvent::Wheel { dx: delta, dy: 0 }
                };
                self.emit(CaptureEvent::Input(event));
                true
            }
            _ => false,
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam as *const KBDLLHOOKSTRUCT);
        let swallow = HOOK_STATE.with(|cell| match cell.try_borrow_mut() {
            Ok(mut guard) => guard.as_mut().is_some_and(|state| state.on_key(info, wparam as u32)),
            Err(_) => false,
        });
        if swallow {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        let swallow = HOOK_STATE.with(|cell| match cell.try_borrow_mut() {
            Ok(mut guard) => guard.as_mut().is_some_and(|state| state.on_mouse(info, wparam as u32)),
            Err(_) => false,
        });
        if swallow {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

pub struct HookThread {
    thread_id: u32,
    join: Option<JoinHandle<()>>,
}

impl HookThread {
    pub fn start(shared: Arc<CaptureShared>, sink: CaptureSink) -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
        let join = std::thread::Builder::new()
            .name("unify-input-hook".into())
            .spawn(move || {
                let hotkey = shared.hotkey_enabled();
                HOOK_STATE.with(|cell| {
                    *cell.borrow_mut() = Some(HookState { shared, sink, filter: CaptureFilter::new(hotkey), last_generation: 0 });
                });
                unsafe {
                    SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
                    let module = GetModuleHandleW(std::ptr::null());
                    let keyboard: HHOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
                    let mouse: HHOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
                    if keyboard.is_null() || mouse.is_null() {
                        let error = std::io::Error::last_os_error().to_string();
                        if !keyboard.is_null() {
                            UnhookWindowsHookEx(keyboard);
                        }
                        if !mouse.is_null() {
                            UnhookWindowsHookEx(mouse);
                        }
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                    let _ = ready_tx.send(Ok(GetCurrentThreadId()));
                    let mut message: MSG = std::mem::zeroed();
                    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    UnhookWindowsHookEx(keyboard);
                    UnhookWindowsHookEx(mouse);
                }
                HOOK_STATE.with(|cell| {
                    if let Ok(mut guard) = cell.try_borrow_mut() {
                        *guard = None;
                    }
                });
            })
            .map_err(|err| PlatformError::Os(err.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(thread_id)) => Ok(Self { thread_id, join: Some(join) }),
            Ok(Err(error)) => {
                let _ = join.join();
                Err(PlatformError::Os(format!("SetWindowsHookEx failed: {error}")))
            }
            Err(_) => Err(PlatformError::Os(String::from("input hook thread exited during startup"))),
        }
    }
}

impl StopHandle for HookThread {
    fn stop(mut self: Box<Self>) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
