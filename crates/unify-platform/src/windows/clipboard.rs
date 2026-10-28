use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Once;
use std::thread::JoinHandle;
use std::time::Duration;

use windows_sys::Win32::Foundation::{GlobalFree, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, RemoveClipboardFormatListener, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostQuitMessage, PostThreadMessageW,
    RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE, WM_DESTROY, WM_QUIT, WNDCLASSW,
};

use crate::{ChangeSink, ClipboardText, PlatformError, StopHandle};

const OPEN_ATTEMPTS: u32 = 10;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(15);

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

thread_local! {
    static WATCH_SINK: RefCell<Option<ChangeSink>> = const { RefCell::new(None) };
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_CLIPBOARDUPDATE => {
            let sink = WATCH_SINK.with(|cell| cell.try_borrow().ok().and_then(|guard| guard.clone()));
            if let Some(sink) = sink {
                sink();
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn class_name() -> Vec<u16> {
    wide("UnifyClipboardWindow")
}

fn register_class() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        let name = class_name();
        let class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: name.as_ptr(),
        };
        RegisterClassW(&class);
    });
}

fn create_message_window() -> Result<HWND, PlatformError> {
    register_class();
    let name = class_name();
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            name.as_ptr(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return Err(PlatformError::Os(format!("CreateWindowExW failed: {}", std::io::Error::last_os_error())));
    }
    Ok(hwnd)
}

struct OpenedClipboard;

impl OpenedClipboard {
    fn open(owner: HWND) -> Result<Self, PlatformError> {
        for attempt in 0..OPEN_ATTEMPTS {
            if unsafe { OpenClipboard(owner) } != 0 {
                return Ok(Self);
            }
            if attempt + 1 < OPEN_ATTEMPTS {
                std::thread::sleep(OPEN_RETRY_DELAY);
            }
        }
        Err(PlatformError::ClipboardBusy)
    }
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

fn registered_format(name: &str) -> u32 {
    let name = wide(name);
    unsafe { RegisterClipboardFormatW(name.as_ptr()) }
}

fn read_dword_format(format: u32) -> Option<u32> {
    if format == 0 || unsafe { IsClipboardFormatAvailable(format) } == 0 {
        return None;
    }
    unsafe {
        let handle = GetClipboardData(format);
        if handle.is_null() {
            return None;
        }
        let pointer = GlobalLock(handle) as *const u32;
        if pointer.is_null() {
            return None;
        }
        let value = if GlobalSize(handle) >= 4 { Some(pointer.read_unaligned()) } else { None };
        GlobalUnlock(handle);
        value
    }
}

fn is_concealed() -> bool {
    let exclude = registered_format("ExcludeClipboardContentFromMonitorProcessing");
    let viewer_ignore = registered_format("Clipboard Viewer Ignore");
    let history = registered_format("CanIncludeInClipboardHistory");
    let flagged = [exclude, viewer_ignore].iter().any(|format| *format != 0 && unsafe { IsClipboardFormatAvailable(*format) } != 0);
    flagged || read_dword_format(history) == Some(0)
}

pub fn read_text() -> Result<Option<ClipboardText>, PlatformError> {
    let _guard = OpenedClipboard::open(std::ptr::null_mut())?;
    if unsafe { IsClipboardFormatAvailable(u32::from(CF_UNICODETEXT)) } == 0 {
        return Ok(None);
    }
    let concealed = is_concealed();
    unsafe {
        let handle = GetClipboardData(u32::from(CF_UNICODETEXT));
        if handle.is_null() {
            return Ok(None);
        }
        let pointer = GlobalLock(handle) as *const u16;
        if pointer.is_null() {
            return Ok(None);
        }
        let capacity = GlobalSize(handle) / 2;
        let units = std::slice::from_raw_parts(pointer, capacity);
        let len = units.iter().position(|unit| *unit == 0).unwrap_or(capacity);
        let text = String::from_utf16_lossy(&units[..len]);
        GlobalUnlock(handle);
        Ok(Some(ClipboardText { text, concealed }))
    }
}

fn write_with_owner(owner: HWND, text: &str) -> Result<(), PlatformError> {
    let encoded = wide(text);
    let bytes = encoded.len() * 2;
    let _guard = OpenedClipboard::open(owner)?;
    unsafe {
        if EmptyClipboard() == 0 {
            return Err(PlatformError::Os(format!("EmptyClipboard failed: {}", std::io::Error::last_os_error())));
        }
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if memory.is_null() {
            return Err(PlatformError::Os(String::from("GlobalAlloc failed")));
        }
        let pointer = GlobalLock(memory) as *mut u16;
        if pointer.is_null() {
            GlobalFree(memory);
            return Err(PlatformError::Os(String::from("GlobalLock failed")));
        }
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), pointer, encoded.len());
        GlobalUnlock(memory);
        if SetClipboardData(u32::from(CF_UNICODETEXT), memory as HANDLE).is_null() {
            GlobalFree(memory);
            return Err(PlatformError::Os(format!("SetClipboardData failed: {}", std::io::Error::last_os_error())));
        }
    }
    Ok(())
}

pub fn write_text(text: &str) -> Result<(), PlatformError> {
    let owner = create_message_window()?;
    let result = write_with_owner(owner, text);
    unsafe {
        DestroyWindow(owner);
    }
    result
}

pub struct ClipboardWatcher {
    thread_id: u32,
    join: Option<JoinHandle<()>>,
}

impl ClipboardWatcher {
    pub fn start(sink: ChangeSink) -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, String>>();
        let join = std::thread::Builder::new()
            .name("unify-clipboard".into())
            .spawn(move || {
                WATCH_SINK.with(|cell| *cell.borrow_mut() = Some(sink));
                let hwnd = match create_message_window() {
                    Ok(hwnd) => hwnd,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err.to_string()));
                        return;
                    }
                };
                unsafe {
                    if AddClipboardFormatListener(hwnd) == 0 {
                        let _ = ready_tx.send(Err(std::io::Error::last_os_error().to_string()));
                        DestroyWindow(hwnd);
                        return;
                    }
                    let _ = ready_tx.send(Ok(GetCurrentThreadId()));
                    let mut message: MSG = std::mem::zeroed();
                    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    RemoveClipboardFormatListener(hwnd);
                    DestroyWindow(hwnd);
                }
                WATCH_SINK.with(|cell| {
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
                Err(PlatformError::Os(format!("clipboard listener failed: {error}")))
            }
            Err(_) => Err(PlatformError::Os(String::from("clipboard thread exited during startup"))),
        }
    }
}

impl StopHandle for ClipboardWatcher {
    fn stop(mut self: Box<Self>) {
        unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    #[ignore = "touches the real Windows clipboard; run with --ignored on a desktop session"]
    fn real_clipboard_roundtrip_preserves_turkish_text_and_line_endings_and_notifies() {
        let counter = Arc::new(AtomicUsize::new(0));
        let observed = counter.clone();
        let watcher = Box::new(ClipboardWatcher::start(Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        }))
        .unwrap());
        let original = read_text().ok().flatten();
        let text = "Unify test: İğüşöç ÇĞÜŞÖ\r\nikinci satır\n🙂";
        write_text(text).unwrap();
        let read = read_text().unwrap().unwrap();
        assert_eq!(read.text, text);
        assert!(!read.concealed);
        for _ in 0..50 {
            if counter.load(Ordering::SeqCst) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(counter.load(Ordering::SeqCst) > 0, "WM_CLIPBOARDUPDATE was not delivered");
        watcher.stop();
        if let Some(original) = original {
            let _ = write_text(&original.text);
        }
    }
}
