use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use unify_proto::keymap::windows_scancode_to_usage;
use unify_proto::{InputEvent, MouseButton};
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
use windows::Win32::Media::MediaFoundation::{IMFDXGIBuffer, IMFSample, MFShutdown, MFStartup, MFSTARTUP_NOSOCKET, MF_VERSION};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW, GetWindowLongPtrW, LoadCursorW,
    PostMessageW, PostQuitMessage, RegisterClassW, SetWindowLongPtrW, ShowWindow, TranslateMessage, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW,
    MSG, SW_SHOWMAXIMIZED, WM_APP, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

use super::decoder::{os_error, Decoder};
use super::renderer::{create_device, Renderer};
use crate::viewer_logic::{fit_rect, pointer_ratio};
use crate::{EncodedFrame, PlatformError, Viewer, ViewerEvent, ViewerSink};

const WM_VIEWER_STOP: u32 = WM_APP + 41;
const CLASS_NAME: &str = "UnifyScreenViewer";
const FRAME_QUEUE: usize = 16;

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn now_us() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0)
}

struct WindowState {
    sink: ViewerSink,
    stream: (u32, u32),
    client_width: Arc<AtomicI32>,
    client_height: Arc<AtomicI32>,
    last_pointer: Option<(u16, u16)>,
}

fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { GetClientRect(hwnd, &mut rect) };
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn scancode_from(lparam: LPARAM) -> u16 {
    let scan = ((lparam >> 16) & 0xFF) as u16;
    if (lparam >> 24) & 1 != 0 {
        0xE000 | scan
    } else {
        scan
    }
}

impl WindowState {
    fn pointer(&mut self, hwnd: HWND, lparam: LPARAM) {
        let x = (lparam & 0xFFFF) as i16 as i32;
        let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
        let (width, height) = client_size(hwnd);
        self.client_width.store(width, Ordering::Relaxed);
        self.client_height.store(height, Ordering::Relaxed);
        let fit = fit_rect(self.stream, (width.max(1) as u32, height.max(1) as u32));
        let ratio = pointer_ratio(fit, x, y);
        if self.last_pointer != Some(ratio) {
            self.last_pointer = Some(ratio);
            (self.sink)(ViewerEvent::Pointer { x: ratio.0, y: ratio.1 });
        }
    }

    fn button(&self, button: MouseButton, down: bool) {
        (self.sink)(ViewerEvent::Input(InputEvent::MouseButton { button, down }));
    }

    fn key(&self, lparam: LPARAM, down: bool) {
        if let Some(usage) = windows_scancode_to_usage(scancode_from(lparam)) {
            (self.sink)(ViewerEvent::Input(InputEvent::Key { usage, down }));
        }
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
    let Some(state) = state.as_mut() else {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    };
    let wheel = ((wparam >> 16) & 0xFFFF) as u16 as i16 as i32;
    let extra = ((wparam >> 16) & 0xFFFF) as u16;
    match message {
        WM_MOUSEMOVE => state.pointer(hwnd, lparam),
        WM_LBUTTONDOWN | WM_LBUTTONUP => {
            state.pointer(hwnd, lparam);
            state.button(MouseButton::Left, message == WM_LBUTTONDOWN);
        }
        WM_RBUTTONDOWN | WM_RBUTTONUP => {
            state.pointer(hwnd, lparam);
            state.button(MouseButton::Right, message == WM_RBUTTONDOWN);
        }
        WM_MBUTTONDOWN | WM_MBUTTONUP => state.button(MouseButton::Middle, message == WM_MBUTTONDOWN),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let button = if extra == 1 { MouseButton::Back } else { MouseButton::Forward };
            state.button(button, message == WM_XBUTTONDOWN);
        }
        WM_MOUSEWHEEL => (state.sink)(ViewerEvent::Input(InputEvent::Wheel { dx: 0, dy: wheel })),
        WM_MOUSEHWHEEL => (state.sink)(ViewerEvent::Input(InputEvent::Wheel { dx: wheel, dy: 0 })),
        WM_KEYDOWN | WM_SYSKEYDOWN => state.key(lparam, true),
        WM_KEYUP | WM_SYSKEYUP => state.key(lparam, false),
        WM_KILLFOCUS => (state.sink)(ViewerEvent::FocusLost),
        WM_ERASEBKGND => return 1,
        WM_CLOSE => (state.sink)(ViewerEvent::Closed),
        WM_VIEWER_STOP => {
            DestroyWindow(hwnd);
        }
        WM_DESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(state));
            PostQuitMessage(0);
        }
        _ => return DefWindowProcW(hwnd, message, wparam, lparam),
    }
    0
}

fn create_window(title: &str, state: Box<WindowState>) -> Result<HWND, PlatformError> {
    let class_name = wide(CLASS_NAME);
    let title = wide(title);
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return Err(PlatformError::Os(format!("CreateWindowExW failed: {}", std::io::Error::last_os_error())));
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
        ShowWindow(hwnd, SW_SHOWMAXIMIZED);
        Ok(hwnd)
    }
}

fn texture_of(sample: &IMFSample) -> Result<(ID3D11Texture2D, u32), PlatformError> {
    unsafe {
        let buffer = sample.GetBufferByIndex(0).map_err(|err| os_error("decoded buffer", err))?;
        let dxgi: IMFDXGIBuffer = buffer.cast().map_err(|err| os_error("decoded surface", err))?;
        let mut raw = std::ptr::null_mut();
        dxgi.GetResource(&ID3D11Texture2D::IID, &mut raw).map_err(|err| os_error("decoded texture", err))?;
        let texture = ID3D11Texture2D::from_raw(raw);
        let subresource = dxgi.GetSubresourceIndex().map_err(|err| os_error("decoded subresource", err))?;
        Ok((texture, subresource))
    }
}

fn render_loop(hwnd: isize, stream: (u32, u32), frames: mpsc::Receiver<EncodedFrame>, sink: ViewerSink, width: Arc<AtomicI32>, height: Arc<AtomicI32>) -> Result<(), PlatformError> {
    let (device, context) = create_device()?;
    let client = (width.load(Ordering::Relaxed).max(1) as u32, height.load(Ordering::Relaxed).max(1) as u32);
    let window = windows::Win32::Foundation::HWND(hwnd as *mut _);
    let mut renderer = Renderer::new(&device, &context, window, stream, client)?;
    let mut decoder = Decoder::new(&device, stream.0, stream.1)?;
    while let Ok(frame) = frames.recv() {
        match decoder.decode(&frame.data) {
            Ok(Some(sample)) => {
                let (texture, subresource) = texture_of(&sample)?;
                renderer.resize((width.load(Ordering::Relaxed).max(1) as u32, height.load(Ordering::Relaxed).max(1) as u32))?;
                renderer.present(&texture, subresource)?;
                sink(ViewerEvent::Presented { capture_us: frame.capture_us, presented_us: now_us() });
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, "frame could not be decoded");
                sink(ViewerEvent::KeyframeNeeded);
            }
        }
    }
    Ok(())
}

pub struct WindowsViewer {
    frames: Option<mpsc::SyncSender<EncodedFrame>>,
    hwnd: AtomicIsize,
    window_thread: Option<JoinHandle<()>>,
    render_thread: Option<JoinHandle<()>>,
}

impl WindowsViewer {
    pub fn start(title: &str, stream: (u32, u32), sink: ViewerSink) -> Result<Self, PlatformError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<isize, String>>();
        let width = Arc::new(AtomicI32::new(1));
        let height = Arc::new(AtomicI32::new(1));
        let (window_width, window_height) = (width.clone(), height.clone());
        let window_sink = sink.clone();
        let title = title.to_owned();
        let window_thread = std::thread::Builder::new()
            .name("unify-viewer-window".into())
            .spawn(move || {
                let state = Box::new(WindowState {
                    sink: window_sink,
                    stream,
                    client_width: window_width.clone(),
                    client_height: window_height.clone(),
                    last_pointer: None,
                });
                let hwnd = match create_window(&title, state) {
                    Ok(hwnd) => hwnd,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err.to_string()));
                        return;
                    }
                };
                let (w, h) = client_size(hwnd);
                window_width.store(w, Ordering::Relaxed);
                window_height.store(h, Ordering::Relaxed);
                let _ = ready_tx.send(Ok(hwnd as isize));
                unsafe {
                    let mut message: MSG = std::mem::zeroed();
                    while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
            })
            .map_err(|err| PlatformError::Os(err.to_string()))?;
        let hwnd = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| PlatformError::Os(String::from("viewer window did not start")))?
            .map_err(PlatformError::Os)?;
        let (frames_tx, frames_rx) = mpsc::sync_channel(FRAME_QUEUE);
        let render_sink = sink;
        let render_thread = std::thread::Builder::new()
            .name("unify-viewer-render".into())
            .spawn(move || unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
                let _ = MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET);
                if let Err(err) = render_loop(hwnd, stream, frames_rx, render_sink.clone(), width, height) {
                    tracing::error!(error = %err, "viewer stopped");
                    render_sink(ViewerEvent::Failed);
                }
                let _ = MFShutdown();
                CoUninitialize();
            })
            .map_err(|err| PlatformError::Os(err.to_string()))?;
        Ok(Self {
            frames: Some(frames_tx),
            hwnd: AtomicIsize::new(hwnd),
            window_thread: Some(window_thread),
            render_thread: Some(render_thread),
        })
    }
}

impl Viewer for WindowsViewer {
    fn submit(&self, frame: EncodedFrame) -> bool {
        self.frames.as_ref().is_some_and(|frames| frames.try_send(frame).is_ok())
    }

    fn stop(mut self: Box<Self>) {
        self.frames = None;
        if let Some(thread) = self.render_thread.take() {
            let _ = thread.join();
        }
        unsafe {
            PostMessageW(self.hwnd.load(Ordering::Relaxed) as HWND, WM_VIEWER_STOP, 0, 0);
        }
        if let Some(thread) = self.window_thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use windows::Win32::Media::MediaFoundation::{
        IMFTransform, MFCreateMediaType, MFCreateMemoryBuffer, MFCreateSample, MFMediaType_Video, MFVideoFormat_H264, MFVideoFormat_NV12,
        CLSID_MSH264EncoderMFT, MFT_OUTPUT_DATA_BUFFER, MF_E_TRANSFORM_NEED_MORE_INPUT, MF_MT_AVG_BITRATE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
        MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

    use super::*;

    const WIDTH: u32 = 1280;
    const HEIGHT: u32 = 720;

    fn encoder() -> IMFTransform {
        unsafe {
            let transform: IMFTransform = CoCreateInstance(&CLSID_MSH264EncoderMFT, None, CLSCTX_INPROC_SERVER).unwrap();
            let size = (u64::from(WIDTH) << 32) | u64::from(HEIGHT);
            let rate = (60u64 << 32) | 1;
            let output = MFCreateMediaType().unwrap();
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).unwrap();
            output.SetUINT32(&MF_MT_AVG_BITRATE, 8_000_000).unwrap();
            output.SetUINT64(&MF_MT_FRAME_SIZE, size).unwrap();
            output.SetUINT64(&MF_MT_FRAME_RATE, rate).unwrap();
            output.SetUINT32(&MF_MT_INTERLACE_MODE, 2).unwrap();
            transform.SetOutputType(0, &output, 0).unwrap();
            let input = MFCreateMediaType().unwrap();
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12).unwrap();
            input.SetUINT64(&MF_MT_FRAME_SIZE, size).unwrap();
            input.SetUINT64(&MF_MT_FRAME_RATE, rate).unwrap();
            input.SetUINT32(&MF_MT_INTERLACE_MODE, 2).unwrap();
            transform.SetInputType(0, &input, 0).unwrap();
            transform
        }
    }

    fn encode(transform: &IMFTransform, frame: u32) -> Vec<Vec<u8>> {
        let luma = (WIDTH * HEIGHT) as usize;
        let mut pixels = vec![128u8; luma + luma / 2];
        let block = (frame * 8 % WIDTH.saturating_sub(120)) as usize;
        for row in 200..320usize {
            pixels[row * WIDTH as usize + block..row * WIDTH as usize + block + 120].fill(235);
        }
        unsafe {
            let buffer = MFCreateMemoryBuffer(pixels.len() as u32).unwrap();
            let mut pointer = std::ptr::null_mut();
            buffer.Lock(&mut pointer, None, None).unwrap();
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), pointer, pixels.len());
            buffer.Unlock().unwrap();
            buffer.SetCurrentLength(pixels.len() as u32).unwrap();
            let sample = MFCreateSample().unwrap();
            sample.AddBuffer(&buffer).unwrap();
            sample.SetSampleTime(i64::from(frame) * 166_667).unwrap();
            sample.SetSampleDuration(166_667).unwrap();
            transform.ProcessInput(0, &sample, 0).unwrap();
            let mut encoded = Vec::new();
            loop {
                let out_buffer = MFCreateMemoryBuffer(1 << 22).unwrap();
                let out_sample = MFCreateSample().unwrap();
                out_sample.AddBuffer(&out_buffer).unwrap();
                let mut output = [MFT_OUTPUT_DATA_BUFFER { pSample: std::mem::ManuallyDrop::new(Some(out_sample.clone())), ..Default::default() }];
                let mut status = 0;
                let result = transform.ProcessOutput(0, &mut output, &mut status);
                drop(std::mem::ManuallyDrop::take(&mut output[0].pSample));
                drop(std::mem::ManuallyDrop::take(&mut output[0].pEvents));
                match result {
                    Ok(()) => {
                        let mut data = std::ptr::null_mut();
                        let mut length = 0;
                        out_buffer.Lock(&mut data, None, Some(&mut length)).unwrap();
                        encoded.push(std::slice::from_raw_parts(data, length as usize).to_vec());
                        out_buffer.Unlock().unwrap();
                    }
                    Err(err) if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => return encoded,
                    Err(err) => panic!("encoder failed: {err}"),
                }
            }
        }
    }

    #[test]
    #[ignore = "opens a window and uses the GPU; run with --ignored on a desktop session"]
    fn decodes_and_presents_real_h264_with_measured_latency() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).unwrap();
        }
        let samples: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
        let failed = Arc::new(Mutex::new(0u32));
        let (recorded, failures) = (samples.clone(), failed.clone());
        let sink: ViewerSink = Arc::new(move |event| match event {
            ViewerEvent::Presented { capture_us, presented_us } => recorded.lock().unwrap().push(presented_us.saturating_sub(capture_us)),
            ViewerEvent::Failed | ViewerEvent::KeyframeNeeded => *failures.lock().unwrap() += 1,
            _ => {}
        });
        let viewer = Box::new(WindowsViewer::start("Unify self-test", (WIDTH, HEIGHT), sink).unwrap());
        let transform = encoder();
        for frame in 0..180 {
            for data in encode(&transform, frame) {
                assert!(viewer.submit(EncodedFrame { data, keyframe: false, capture_us: now_us() }), "queue full");
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        std::thread::sleep(Duration::from_millis(300));
        Box::new(*viewer).stop();
        let mut values = samples.lock().unwrap().clone();
        values.sort_unstable();
        assert_eq!(*failed.lock().unwrap(), 0, "decoder or renderer reported failures");
        assert!(values.len() > 120, "only {} frames presented", values.len());
        println!(
            "decode+present over {} frames: p50 {:.2} ms, p99 {:.2} ms, max {:.2} ms",
            values.len(),
            values[values.len() / 2] as f64 / 1000.0,
            values[values.len() * 99 / 100] as f64 / 1000.0,
            values[values.len() - 1] as f64 / 1000.0
        );
    }
}
