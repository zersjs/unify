use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use block2::RcBlock;
use core_graphics::display::{CGConfigureOption, CGDisplay, CGDisplayMode};
use dispatch2::{DispatchQueue, DispatchRetained};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass};
use objc2_core_foundation::{kCFBooleanTrue, CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_media::{kCMSampleAttachmentKey_NotSync, kCMTimeInvalid, CMSampleBuffer, CMTime, CMVideoFormatDescriptionGetH264ParameterSetAtIndex};
use objc2_core_video::{kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, CVImageBuffer};
use objc2_foundation::{NSError, NSObjectProtocol};
use objc2_screen_capture_kit::{
    SCContentFilter, SCShareableContent, SCStream, SCStreamConfiguration, SCStreamOutput, SCStreamOutputType, SCWindow,
};
use objc2_video_toolbox::{
    kVTCompressionPropertyKey_AllowFrameReordering, kVTCompressionPropertyKey_AverageBitRate, kVTCompressionPropertyKey_ExpectedFrameRate,
    kVTCompressionPropertyKey_MaxKeyFrameInterval, kVTCompressionPropertyKey_ProfileLevel, kVTCompressionPropertyKey_RealTime,
    kVTEncodeFrameOptionKey_ForceKeyFrame, kVTProfileLevel_H264_High_AutoLevel, kVTVideoEncoderSpecification_EnableLowLatencyRateControl,
    VTCompressionSession, VTEncodeInfoFlags, VTSession, VTSessionSetProperty,
};

use crate::mac_logic::avcc_to_annex_b;
use crate::viewer_logic::pick_display_mode;
use crate::{EncodedFrame, FrameSink, PlatformError, ScreenCapture, ScreenSettings, StopHandle};

const CODEC_H264: u32 = u32::from_be_bytes(*b"avc1");
const START_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_KEYFRAME_INTERVAL_FRAMES: i32 = 600;
const QUEUE_DEPTH: isize = 3;

pub fn match_display_mode(width: u32, height: u32) -> Result<(u32, u32), PlatformError> {
    let display = CGDisplay::main();
    let current = display.display_mode().ok_or_else(|| PlatformError::Os(String::from("no current display mode")))?;
    let current_size = (current.pixel_width() as u32, current.pixel_height() as u32);
    let modes = CGDisplayMode::all_display_modes(display.id, std::ptr::null()).unwrap_or_default();
    let sizes: Vec<(u32, u32)> = modes.iter().map(|mode| (mode.pixel_width() as u32, mode.pixel_height() as u32)).collect();
    let Some(chosen) = pick_display_mode(&sizes, (width, height)).filter(|index| sizes[*index] != current_size) else {
        return Ok(current_size);
    };
    let config = display.begin_configuration().map_err(|code| PlatformError::Os(format!("CGBeginDisplayConfiguration failed: {code}")))?;
    if let Err(code) = display.configure_display_with_display_mode(&config, &modes[chosen]) {
        let _ = display.cancel_configuration(&config);
        return Err(PlatformError::Os(format!("CGConfigureDisplayWithDisplayMode failed: {code}")));
    }
    display
        .complete_configuration(&config, CGConfigureOption::ConfigureForAppOnly)
        .map_err(|code| PlatformError::Os(format!("CGCompleteDisplayConfiguration failed: {code}")))?;
    Ok(sizes[chosen])
}

pub fn restore_display_mode() {
    unsafe { core_graphics::display::CGRestorePermanentDisplayConfiguration() };
}

fn now_us() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0)
}

struct EncoderContext {
    sink: FrameSink,
}

struct Encoder {
    session: CFRetained<VTCompressionSession>,
    context: *mut EncoderContext,
    force_key: Arc<AtomicBool>,
    last_image: Mutex<Option<SendImage>>,
    last_pts: Mutex<CMTime>,
}

struct SendImage(CFRetained<CVImageBuffer>);
unsafe impl Send for SendImage {}
unsafe impl Send for Encoder {}
unsafe impl Sync for Encoder {}

unsafe extern "C-unwind" fn encoder_output(
    context: *mut c_void,
    source: *mut c_void,
    status: i32,
    _flags: VTEncodeInfoFlags,
    sample: *mut CMSampleBuffer,
) {
    let (Some(context), Some(sample)) = (unsafe { (context as *const EncoderContext).as_ref() }, unsafe { sample.as_ref() }) else {
        return;
    };
    if status != 0 {
        tracing::warn!(status, "video encoder reported an error");
        return;
    }
    let Some(block) = (unsafe { sample.data_buffer() }) else {
        return;
    };
    let mut pointer: *mut std::ffi::c_char = std::ptr::null_mut();
    let mut total = 0usize;
    if unsafe { block.data_pointer(0, std::ptr::null_mut(), &mut total, &mut pointer) } != 0 || pointer.is_null() {
        return;
    }
    let avcc = unsafe { std::slice::from_raw_parts(pointer as *const u8, total) };
    let keyframe = is_keyframe(sample);
    let parameter_sets = if keyframe { parameter_sets(sample) } else { Vec::new() };
    let Some(data) = avcc_to_annex_b(avcc, &parameter_sets) else {
        tracing::warn!("encoder produced a malformed frame");
        return;
    };
    (context.sink)(EncodedFrame { data, keyframe, capture_us: source as u64 });
}

fn is_keyframe(sample: &CMSampleBuffer) -> bool {
    let Some(attachments) = (unsafe { sample.sample_attachments_array(false) }) else {
        return true;
    };
    let attachments: &CFArray<CFDictionary<CFString, CFType>> = unsafe { attachments.cast_unchecked() };
    let Some(first) = attachments.get(0) else {
        return true;
    };
    !first.contains_key(unsafe { kCMSampleAttachmentKey_NotSync })
}

fn parameter_sets(sample: &CMSampleBuffer) -> Vec<Vec<u8>> {
    let Some(format) = (unsafe { sample.format_description() }) else {
        return Vec::new();
    };
    let mut count = 0usize;
    if unsafe {
        CMVideoFormatDescriptionGetH264ParameterSetAtIndex(&format, 0, std::ptr::null_mut(), std::ptr::null_mut(), &mut count, std::ptr::null_mut())
    } != 0
    {
        return Vec::new();
    }
    (0..count)
        .filter_map(|index| {
            let mut pointer: *const u8 = std::ptr::null();
            let mut size = 0usize;
            let status = unsafe {
                CMVideoFormatDescriptionGetH264ParameterSetAtIndex(&format, index, &mut pointer, &mut size, std::ptr::null_mut(), std::ptr::null_mut())
            };
            (status == 0 && !pointer.is_null()).then(|| unsafe { std::slice::from_raw_parts(pointer, size) }.to_vec())
        })
        .collect()
}

fn set_property(session: &VTCompressionSession, key: &CFString, value: &CFType) -> Result<(), PlatformError> {
    let target = unsafe { &*(session as *const VTCompressionSession as *const VTSession) };
    let status = unsafe { VTSessionSetProperty(target, key, Some(value)) };
    if status != 0 {
        return Err(PlatformError::Os(format!("VTSessionSetProperty failed: {status}")));
    }
    Ok(())
}

impl Encoder {
    fn new(settings: ScreenSettings, sink: FrameSink) -> Result<Self, PlatformError> {
        let true_value: &CFBoolean = unsafe { kCFBooleanTrue }.ok_or_else(|| PlatformError::Os(String::from("kCFBooleanTrue missing")))?;
        let spec_key = unsafe { kVTVideoEncoderSpecification_EnableLowLatencyRateControl };
        let specification = CFDictionary::<CFString, CFBoolean>::from_slices(&[spec_key], &[true_value]);
        let context = Box::into_raw(Box::new(EncoderContext { sink }));
        let mut raw: *mut VTCompressionSession = std::ptr::null_mut();
        let status = unsafe {
            VTCompressionSession::create(
                None,
                settings.width as i32,
                settings.height as i32,
                CODEC_H264,
                Some(specification.as_opaque()),
                None,
                None,
                Some(encoder_output),
                context.cast(),
                NonNull::from(&mut raw),
            )
        };
        let Some(pointer) = NonNull::new(raw).filter(|_| status == 0) else {
            drop(unsafe { Box::from_raw(context) });
            return Err(PlatformError::Os(format!("VTCompressionSessionCreate failed: {status}")));
        };
        let session = unsafe { CFRetained::from_raw(pointer) };
        let bitrate = CFNumber::new_i32(settings.bitrate_bps as i32);
        let fps = CFNumber::new_i32(settings.fps as i32);
        let interval = CFNumber::new_i32(MAX_KEYFRAME_INTERVAL_FRAMES);
        let false_value = unsafe { objc2_core_foundation::kCFBooleanFalse }.ok_or_else(|| PlatformError::Os(String::from("kCFBooleanFalse missing")))?;
        unsafe {
            set_property(&session, kVTCompressionPropertyKey_RealTime, true_value)?;
            set_property(&session, kVTCompressionPropertyKey_AllowFrameReordering, false_value)?;
            set_property(&session, kVTCompressionPropertyKey_ProfileLevel, kVTProfileLevel_H264_High_AutoLevel)?;
            set_property(&session, kVTCompressionPropertyKey_AverageBitRate, &bitrate)?;
            set_property(&session, kVTCompressionPropertyKey_ExpectedFrameRate, &fps)?;
            set_property(&session, kVTCompressionPropertyKey_MaxKeyFrameInterval, &interval)?;
            session.prepare_to_encode_frames();
        }
        Ok(Self {
            session,
            context,
            force_key: Arc::new(AtomicBool::new(true)),
            last_image: Mutex::new(None),
            last_pts: Mutex::new(unsafe { kCMTimeInvalid }),
        })
    }

    fn encode(&self, image: &CVImageBuffer, pts: CMTime) {
        let force = self.force_key.swap(false, Ordering::SeqCst);
        let properties = force.then(|| {
            let key = unsafe { kVTEncodeFrameOptionKey_ForceKeyFrame };
            let value: &CFBoolean = unsafe { kCFBooleanTrue }.unwrap_or_else(|| unreachable!("CoreFoundation constant"));
            CFDictionary::<CFString, CFBoolean>::from_slices(&[key], &[value])
        });
        let status = unsafe {
            self.session.encode_frame(
                image,
                pts,
                kCMTimeInvalid,
                properties.as_ref().map(|dictionary| dictionary.as_opaque()),
                now_us() as *mut c_void,
                std::ptr::null_mut(),
            )
        };
        if status != 0 {
            tracing::warn!(status, "VTCompressionSessionEncodeFrame failed");
        }
    }

    fn on_frame(&self, sample: &CMSampleBuffer) {
        let Some(image) = (unsafe { sample.image_buffer() }) else {
            return;
        };
        let pts = unsafe { sample.presentation_time_stamp() };
        self.encode(&image, pts);
        if let Ok(mut last) = self.last_image.lock() {
            *last = Some(SendImage(image));
        }
        if let Ok(mut last) = self.last_pts.lock() {
            *last = pts;
        }
    }

    fn repeat_last_as_keyframe(&self) {
        let image = self.last_image.lock().ok().and_then(|last| last.as_ref().map(|image| image.0.clone()));
        let pts = self.last_pts.lock().map(|mut last| {
            last.value += i64::from(last.timescale.max(1)) / 1000 + 1;
            *last
        });
        if let (Some(image), Ok(pts)) = (image, pts) {
            self.encode(&image, pts);
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            self.session.invalidate();
            drop(Box::from_raw(self.context));
        }
    }
}

struct OutputIvars {
    encoder: Arc<Encoder>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "UnifyScreenOutput"]
    #[ivars = OutputIvars]
    struct ScreenOutput;

    unsafe impl NSObjectProtocol for ScreenOutput {}

    unsafe impl SCStreamOutput for ScreenOutput {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        unsafe fn stream_did_output(&self, _stream: &SCStream, sample: &CMSampleBuffer, kind: SCStreamOutputType) {
            if kind == SCStreamOutputType::Screen {
                self.ivars().encoder.on_frame(sample);
            }
        }
    }
);

impl ScreenOutput {
    fn new(encoder: Arc<Encoder>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(OutputIvars { encoder });
        unsafe { msg_send![super(this), init] }
    }
}

fn wait<T>(receiver: mpsc::Receiver<Result<T, PlatformError>>) -> Result<T, PlatformError> {
    receiver.recv_timeout(START_TIMEOUT).map_err(|_| PlatformError::Os(String::from("ScreenCaptureKit did not respond")))?
}

fn describe(error: *mut NSError) -> PlatformError {
    match unsafe { error.as_ref() } {
        Some(error) if error.code() == -3801 => {
            unsafe { super::ffi::CGRequestScreenCaptureAccess() };
            PlatformError::PermissionDenied("screen_recording")
        }
        Some(error) => PlatformError::Os(error.localizedDescription().to_string()),
        None => PlatformError::Os(String::from("unknown ScreenCaptureKit error")),
    }
}

fn shareable_content() -> Result<Retained<SCShareableContent>, PlatformError> {
    let (tx, rx) = mpsc::channel();
    let block = RcBlock::new(move |content: *mut SCShareableContent, error: *mut NSError| {
        let result = match unsafe { Retained::retain(content) } {
            Some(content) => Ok(SendContent(content)),
            None => Err(describe(error)),
        };
        let _ = tx.send(result);
    });
    unsafe { SCShareableContent::getShareableContentWithCompletionHandler(&block) };
    wait(rx).map(|content| content.0)
}

struct SendContent(Retained<SCShareableContent>);
unsafe impl Send for SendContent {}

pub struct MacScreenCapture {
    stream: Retained<SCStream>,
    _output: Retained<ScreenOutput>,
    encoder: Arc<Encoder>,
    queue: DispatchRetained<DispatchQueue>,
}

unsafe impl Send for MacScreenCapture {}
unsafe impl Sync for MacScreenCapture {}

impl MacScreenCapture {
    pub fn start(settings: ScreenSettings, sink: FrameSink) -> Result<Self, PlatformError> {
        let content = shareable_content()?;
        let displays = unsafe { content.displays() };
        let main_id = CGDisplay::main().id;
        let display = displays
            .iter()
            .find(|display| unsafe { display.displayID() } == main_id)
            .or_else(|| displays.firstObject())
            .ok_or_else(|| PlatformError::Os(String::from("no display to capture")))?;
        let excluded = objc2_foundation::NSArray::<SCWindow>::new();
        let filter = unsafe { SCContentFilter::initWithDisplay_excludingWindows(SCContentFilter::alloc(), &display, &excluded) };
        let configuration = unsafe { SCStreamConfiguration::new() };
        unsafe {
            configuration.setWidth(settings.width as usize);
            configuration.setHeight(settings.height as usize);
            configuration.setMinimumFrameInterval(CMTime::new(1, settings.fps as i32));
            configuration.setPixelFormat(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange);
            configuration.setQueueDepth(QUEUE_DEPTH);
            configuration.setShowsCursor(false);
        }
        let encoder = Arc::new(Encoder::new(settings, sink)?);
        let output = ScreenOutput::new(encoder.clone());
        let queue = DispatchQueue::new("dev.unify.screen", None);
        let stream = unsafe { SCStream::initWithFilter_configuration_delegate(SCStream::alloc(), &filter, &configuration, None) };
        unsafe {
            stream
                .addStreamOutput_type_sampleHandlerQueue_error(ProtocolObject::from_ref(&*output), SCStreamOutputType::Screen, Some(&queue))
                .map_err(|error| PlatformError::Os(error.localizedDescription().to_string()))?;
        }
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |error: *mut NSError| {
            let _ = tx.send(if error.is_null() { Ok(()) } else { Err(describe(error)) });
        });
        unsafe { stream.startCaptureWithCompletionHandler(Some(&block)) };
        wait(rx)?;
        Ok(Self { stream, _output: output, encoder, queue })
    }
}

impl ScreenCapture for MacScreenCapture {
    fn request_keyframe(&self) {
        self.encoder.force_key.store(true, Ordering::SeqCst);
        let encoder = self.encoder.clone();
        self.queue.exec_async(move || encoder.repeat_last_as_keyframe());
    }
}

impl StopHandle for MacScreenCapture {
    fn stop(self: Box<Self>) {
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |_error: *mut NSError| {
            let _ = tx.send(());
        });
        unsafe { self.stream.stopCaptureWithCompletionHandler(Some(&block)) };
        let _ = rx.recv_timeout(START_TIMEOUT);
    }
}
