use unify_proto::keymap::{usage, usage_to_windows_scancode, WIN_PAUSE_SENTINEL};
use unify_proto::MouseButton;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
    MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VK_NUMLOCK, VK_PAUSE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::{InjectEvent, Injector, PlatformError, INJECTION_MARKER};

const XBUTTON1: i32 = 1;
const XBUTTON2: i32 = 2;

pub struct WindowsInjector;

fn keyboard(virtual_key: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: virtual_key, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: INJECTION_MARKER },
        },
    }
}

fn mouse(dx: i32, dy: i32, data: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx, dy, mouseData: data as _, dwFlags: flags, time: 0, dwExtraInfo: INJECTION_MARKER },
        },
    }
}

fn key_input(code: u16, down: bool) -> Option<INPUT> {
    let up_flag = if down { 0 } else { KEYEVENTF_KEYUP };
    if code == usage::PAUSE {
        return Some(keyboard(VK_PAUSE, 0, up_flag));
    }
    if code == usage::NUM_LOCK {
        return Some(keyboard(VK_NUMLOCK, 0, up_flag | KEYEVENTF_EXTENDEDKEY));
    }
    let scan = usage_to_windows_scancode(code)?;
    if scan == WIN_PAUSE_SENTINEL {
        return Some(keyboard(VK_PAUSE, 0, up_flag));
    }
    let extended = if scan & 0xFF00 == 0xE000 { KEYEVENTF_EXTENDEDKEY } else { 0 };
    Some(keyboard(0, scan & 0xFF, KEYEVENTF_SCANCODE | extended | up_flag))
}

fn normalize(value: i32, origin: i32, extent: i32) -> i32 {
    if extent <= 1 {
        return 0;
    }
    let offset = i64::from((value - origin).clamp(0, extent - 1));
    ((offset * 65_536 + i64::from(extent) - 1) / i64::from(extent)).clamp(0, 65_535) as i32
}

fn move_input(x: i32, y: i32) -> INPUT {
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    mouse(normalize(x, vx, vw), normalize(y, vy, vh), 0, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK)
}

fn button_input(button: MouseButton, down: bool) -> INPUT {
    match (button, down) {
        (MouseButton::Left, true) => mouse(0, 0, 0, MOUSEEVENTF_LEFTDOWN),
        (MouseButton::Left, false) => mouse(0, 0, 0, MOUSEEVENTF_LEFTUP),
        (MouseButton::Right, true) => mouse(0, 0, 0, MOUSEEVENTF_RIGHTDOWN),
        (MouseButton::Right, false) => mouse(0, 0, 0, MOUSEEVENTF_RIGHTUP),
        (MouseButton::Middle, true) => mouse(0, 0, 0, MOUSEEVENTF_MIDDLEDOWN),
        (MouseButton::Middle, false) => mouse(0, 0, 0, MOUSEEVENTF_MIDDLEUP),
        (MouseButton::Back, true) => mouse(0, 0, XBUTTON1, MOUSEEVENTF_XDOWN),
        (MouseButton::Back, false) => mouse(0, 0, XBUTTON1, MOUSEEVENTF_XUP),
        (MouseButton::Forward, true) => mouse(0, 0, XBUTTON2, MOUSEEVENTF_XDOWN),
        (MouseButton::Forward, false) => mouse(0, 0, XBUTTON2, MOUSEEVENTF_XUP),
    }
}

fn send(inputs: &[INPUT]) -> Result<(), PlatformError> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(PlatformError::Os(format!("SendInput rejected input: {}", std::io::Error::last_os_error())));
    }
    Ok(())
}

impl Injector for WindowsInjector {
    fn inject(&self, event: InjectEvent) -> Result<(), PlatformError> {
        match event {
            InjectEvent::Key { usage, down } => match key_input(usage, down) {
                Some(input) => send(&[input]),
                None => Ok(()),
            },
            InjectEvent::MoveTo { x, y, .. } => send(&[move_input(x, y)]),
            InjectEvent::Button { button, down } => send(&[button_input(button, down)]),
            InjectEvent::Wheel { dx, dy } => {
                let mut inputs = Vec::with_capacity(2);
                if dy != 0 {
                    inputs.push(mouse(0, 0, dy, MOUSEEVENTF_WHEEL));
                }
                if dx != 0 {
                    inputs.push(mouse(0, 0, dx, MOUSEEVENTF_HWHEEL));
                }
                send(&inputs)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_pixel_to_absolute_range_and_back() {
        for extent in [1920, 2560, 3840, 5120] {
            for x in [0, 1, extent / 2, extent - 2, extent - 1] {
                let n = normalize(x, 0, extent);
                assert!((0..=65_535).contains(&n));
                let back = (i64::from(n) * i64::from(extent) / 65_536) as i32;
                assert_eq!(back, x, "extent {extent} x {x} n {n}");
            }
        }
        assert_eq!(normalize(-5000, -1920, 3840), 0);
    }

    #[test]
    fn maps_extended_and_special_keys() {
        let InputKey { scan, flags, vk } = describe(key_input(0x4F, true).unwrap());
        assert_eq!((scan, vk), (0x4D, 0));
        assert!(flags & KEYEVENTF_EXTENDEDKEY != 0);
        let InputKey { vk, .. } = describe(key_input(usage::PAUSE, true).unwrap());
        assert_eq!(vk, VK_PAUSE);
        let InputKey { flags, .. } = describe(key_input(0x04, false).unwrap());
        assert!(flags & KEYEVENTF_KEYUP != 0);
        assert!(key_input(0x3FF, true).is_none());
    }

    struct InputKey {
        scan: u16,
        flags: u32,
        vk: u16,
    }

    fn describe(input: INPUT) -> InputKey {
        let ki = unsafe { input.Anonymous.ki };
        InputKey { scan: ki.wScan, flags: ki.dwFlags, vk: ki.wVk }
    }
}
