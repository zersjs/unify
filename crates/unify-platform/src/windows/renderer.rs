use std::mem::ManuallyDrop;

use windows::core::{Interface, BOOL};
use windows::Win32::Foundation::{HMODULE, HWND, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Multithread, ID3D11Texture2D, ID3D11VideoContext, ID3D11VideoDevice,
    ID3D11VideoProcessor, ID3D11VideoProcessorEnumerator, ID3D11VideoProcessorOutputView, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_SDK_VERSION, D3D11_TEX2D_VPIV, D3D11_TEX2D_VPOV, D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
    D3D11_VIDEO_PROCESSOR_COLOR_SPACE, D3D11_VIDEO_PROCESSOR_CONTENT_DESC, D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC,
    D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0, D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC, D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0,
    D3D11_VIDEO_PROCESSOR_STREAM, D3D11_VIDEO_USAGE_OPTIMAL_SPEED, D3D11_VPIV_DIMENSION_TEXTURE2D, D3D11_VPOV_DIMENSION_TEXTURE2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_RATIONAL, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory2, IDXGIFactory5, IDXGISwapChain1, DXGI_FEATURE_PRESENT_ALLOW_TEARING, DXGI_PRESENT,
    DXGI_PRESENT_ALLOW_TEARING, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING,
    DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};

use super::decoder::os_error;
use crate::viewer_logic::fit_rect;
use crate::PlatformError;

const BACK_BUFFERS: u32 = 2;
const NOMINAL_FPS: u32 = 60;

pub fn create_device() -> Result<(ID3D11Device, ID3D11DeviceContext), PlatformError> {
    let mut device = None;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
        .map_err(|err| os_error("D3D11CreateDevice", err))?;
    }
    let device: ID3D11Device = device.ok_or_else(|| PlatformError::Os(String::from("D3D11 device missing")))?;
    let context: ID3D11DeviceContext = context.ok_or_else(|| PlatformError::Os(String::from("D3D11 context missing")))?;
    let multithread: ID3D11Multithread = device.cast().map_err(|err| os_error("multithread interface", err))?;
    let _ = unsafe { multithread.SetMultithreadProtected(true) };
    Ok((device, context))
}

pub struct Renderer {
    swap_chain: IDXGISwapChain1,
    tearing: bool,
    video_device: ID3D11VideoDevice,
    video_context: ID3D11VideoContext,
    stream: (u32, u32),
    client: (u32, u32),
    enumerator: Option<ID3D11VideoProcessorEnumerator>,
    processor: Option<ID3D11VideoProcessor>,
    output_view: Option<ID3D11VideoProcessorOutputView>,
}

fn tearing_supported(factory: &IDXGIFactory2) -> bool {
    let Ok(factory5) = factory.cast::<IDXGIFactory5>() else {
        return false;
    };
    let mut allowed = BOOL(0);
    let result = unsafe {
        factory5.CheckFeatureSupport(
            DXGI_FEATURE_PRESENT_ALLOW_TEARING,
            (&mut allowed as *mut BOOL).cast(),
            std::mem::size_of::<BOOL>() as u32,
        )
    };
    result.is_ok() && allowed.as_bool()
}

impl Renderer {
    pub fn new(device: &ID3D11Device, context: &ID3D11DeviceContext, hwnd: HWND, stream: (u32, u32), client: (u32, u32)) -> Result<Self, PlatformError> {
        let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory1() }.map_err(|err| os_error("DXGI factory", err))?;
        let tearing = tearing_supported(&factory);
        let description = DXGI_SWAP_CHAIN_DESC1 {
            Width: client.0.max(1),
            Height: client.1.max(1),
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: BACK_BUFFERS,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Flags: if tearing { DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING.0 as u32 } else { 0 },
            ..Default::default()
        };
        let swap_chain = unsafe { factory.CreateSwapChainForHwnd(device, hwnd, &description, None, None) }.map_err(|err| os_error("swap chain", err))?;
        let video_device: ID3D11VideoDevice = device.cast().map_err(|err| os_error("video device", err))?;
        let video_context: ID3D11VideoContext = context.cast().map_err(|err| os_error("video context", err))?;
        let mut renderer = Self {
            swap_chain,
            tearing,
            video_device,
            video_context,
            stream,
            client: client.max((1, 1)),
            enumerator: None,
            processor: None,
            output_view: None,
        };
        renderer.rebuild()?;
        Ok(renderer)
    }

    fn rebuild(&mut self) -> Result<(), PlatformError> {
        let rate = DXGI_RATIONAL { Numerator: NOMINAL_FPS, Denominator: 1 };
        let content = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: rate,
            InputWidth: self.stream.0,
            InputHeight: self.stream.1,
            OutputFrameRate: rate,
            OutputWidth: self.client.0,
            OutputHeight: self.client.1,
            Usage: D3D11_VIDEO_USAGE_OPTIMAL_SPEED,
        };
        let enumerator = unsafe { self.video_device.CreateVideoProcessorEnumerator(&content) }.map_err(|err| os_error("video processor enumerator", err))?;
        let processor = unsafe { self.video_device.CreateVideoProcessor(&enumerator, 0) }.map_err(|err| os_error("video processor", err))?;
        let back_buffer: ID3D11Texture2D = unsafe { self.swap_chain.GetBuffer(0) }.map_err(|err| os_error("back buffer", err))?;
        let view_description = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
            ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 } },
        };
        let mut output_view = None;
        unsafe { self.video_device.CreateVideoProcessorOutputView(&back_buffer, &enumerator, &view_description, Some(&mut output_view)) }
            .map_err(|err| os_error("output view", err))?;
        let fit = fit_rect(self.stream, self.client);
        let destination = RECT { left: fit.left, top: fit.top, right: fit.left + fit.width, bottom: fit.top + fit.height };
        let color_space = D3D11_VIDEO_PROCESSOR_COLOR_SPACE { _bitfield: 0b0000_0100 | (1 << 4) };
        unsafe {
            self.video_context.VideoProcessorSetStreamDestRect(&processor, 0, true, Some(&destination));
            self.video_context.VideoProcessorSetStreamColorSpace(&processor, 0, &color_space);
            self.video_context.VideoProcessorSetStreamAutoProcessingMode(&processor, 0, false);
        }
        self.enumerator = Some(enumerator);
        self.processor = Some(processor);
        self.output_view = output_view;
        Ok(())
    }

    pub fn resize(&mut self, client: (u32, u32)) -> Result<(), PlatformError> {
        let client = client.max((1, 1));
        if client == self.client {
            return Ok(());
        }
        self.client = client;
        self.output_view = None;
        self.processor = None;
        self.enumerator = None;
        let flags = if self.tearing { DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING } else { DXGI_SWAP_CHAIN_FLAG(0) };
        unsafe { self.swap_chain.ResizeBuffers(0, client.0, client.1, DXGI_FORMAT_B8G8R8A8_UNORM, flags) }.map_err(|err| os_error("resize", err))?;
        self.rebuild()
    }

    pub fn present(&mut self, texture: &ID3D11Texture2D, subresource: u32) -> Result<(), PlatformError> {
        let (Some(enumerator), Some(processor), Some(output_view)) = (&self.enumerator, &self.processor, &self.output_view) else {
            return Ok(());
        };
        let description = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_VPIV { MipSlice: 0, ArraySlice: subresource } },
        };
        let mut input_view = None;
        unsafe { self.video_device.CreateVideoProcessorInputView(texture, enumerator, &description, Some(&mut input_view)) }
            .map_err(|err| os_error("input view", err))?;
        let stream = D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: true.into(),
            pInputSurface: ManuallyDrop::new(input_view),
            ..Default::default()
        };
        let mut streams = [stream];
        let result = unsafe { self.video_context.VideoProcessorBlt(processor, output_view, 0, &streams) };
        unsafe { ManuallyDrop::drop(&mut streams[0].pInputSurface) };
        result.map_err(|err| os_error("VideoProcessorBlt", err))?;
        let flags: DXGI_PRESENT = if self.tearing { DXGI_PRESENT_ALLOW_TEARING } else { DXGI_PRESENT(0) };
        unsafe { self.swap_chain.Present(0, flags) }.ok().map_err(|err| os_error("Present", err))
    }
}
