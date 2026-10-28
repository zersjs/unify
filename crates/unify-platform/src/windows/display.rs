use unify_core::geometry::{Desktop, Rect};
use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

use crate::PlatformError;

unsafe extern "system" fn collect_monitor(_monitor: HMONITOR, _hdc: HDC, rect: *mut RECT, data: LPARAM) -> BOOL {
    let monitors = &mut *(data as *mut Vec<Rect>);
    if let Some(rect) = rect.as_ref() {
        monitors.push(Rect::new(rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
    }
    TRUE
}

pub fn desktop() -> Result<Desktop, PlatformError> {
    let mut monitors: Vec<Rect> = Vec::new();
    let ok = unsafe {
        EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(collect_monitor), &mut monitors as *mut Vec<Rect> as LPARAM)
    };
    if ok == 0 {
        return Err(PlatformError::Os(format!("EnumDisplayMonitors failed: {}", std::io::Error::last_os_error())));
    }
    Desktop::new(monitors).ok_or_else(|| PlatformError::Os(String::from("no active displays")))
}
