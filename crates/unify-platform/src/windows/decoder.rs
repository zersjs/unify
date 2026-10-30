use windows::core::{Interface, GUID};
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Media::MediaFoundation::{
    IMFDXGIDeviceManager, IMFSample, IMFTransform, MFCreateDXGIDeviceManager, MFCreateMediaType, MFCreateMemoryBuffer, MFCreateSample,
    MFMediaType_Video, MFVideoFormat_H264, MFVideoFormat_NV12, MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, MFT_MESSAGE_NOTIFY_START_OF_STREAM,
    MFT_MESSAGE_SET_D3D_MANAGER, MFT_OUTPUT_DATA_BUFFER, MF_E_TRANSFORM_NEED_MORE_INPUT, MF_E_TRANSFORM_STREAM_CHANGE, MF_LOW_LATENCY,
    MF_MT_FRAME_SIZE, MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

use crate::PlatformError;

const CLSID_H264_DECODER: GUID = GUID::from_u128(0x62ce7e72_4c71_4d20_b15d_452831a87d9d);

pub fn os_error(context: &str, error: windows::core::Error) -> PlatformError {
    PlatformError::Os(format!("{context}: {error}"))
}

pub struct Decoder {
    transform: IMFTransform,
    _manager: IMFDXGIDeviceManager,
    sample_time: i64,
}

impl Decoder {
    pub fn new(device: &ID3D11Device, width: u32, height: u32) -> Result<Self, PlatformError> {
        unsafe {
            let transform: IMFTransform =
                CoCreateInstance(&CLSID_H264_DECODER, None, CLSCTX_INPROC_SERVER).map_err(|err| os_error("H.264 decoder", err))?;
            let attributes = transform.GetAttributes().map_err(|err| os_error("decoder attributes", err))?;
            attributes.SetUINT32(&MF_LOW_LATENCY, 1).map_err(|err| os_error("low latency mode", err))?;
            let mut token = 0u32;
            let mut manager: Option<IMFDXGIDeviceManager> = None;
            MFCreateDXGIDeviceManager(&mut token, &mut manager).map_err(|err| os_error("DXGI manager", err))?;
            let manager = manager.ok_or_else(|| PlatformError::Os(String::from("DXGI manager missing")))?;
            manager.ResetDevice(device, token).map_err(|err| os_error("DXGI manager reset", err))?;
            transform
                .ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager.as_raw() as usize)
                .map_err(|err| os_error("hardware decoding", err))?;
            let input = MFCreateMediaType().map_err(|err| os_error("input type", err))?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|err| os_error("input major type", err))?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).map_err(|err| os_error("input subtype", err))?;
            input
                .SetUINT64(&MF_MT_FRAME_SIZE, (u64::from(width) << 32) | u64::from(height))
                .map_err(|err| os_error("input size", err))?;
            transform.SetInputType(0, &input, 0).map_err(|err| os_error("set input type", err))?;
            let decoder = Self { transform, _manager: manager, sample_time: 0 };
            decoder.select_output()?;
            decoder.transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0).map_err(|err| os_error("begin streaming", err))?;
            decoder.transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0).map_err(|err| os_error("start stream", err))?;
            Ok(decoder)
        }
    }

    fn select_output(&self) -> Result<(), PlatformError> {
        for index in 0.. {
            let available = unsafe { self.transform.GetOutputAvailableType(0, index) }.map_err(|err| os_error("output types", err))?;
            if unsafe { available.GetGUID(&MF_MT_SUBTYPE) }.map_err(|err| os_error("output subtype", err))? == MFVideoFormat_NV12 {
                return unsafe { self.transform.SetOutputType(0, &available, 0) }.map_err(|err| os_error("set output type", err));
            }
        }
        Err(PlatformError::Os(String::from("decoder has no NV12 output")))
    }

    fn input_sample(&mut self, data: &[u8]) -> Result<IMFSample, PlatformError> {
        unsafe {
            let buffer = MFCreateMemoryBuffer(data.len() as u32).map_err(|err| os_error("input buffer", err))?;
            let mut pointer = std::ptr::null_mut();
            buffer.Lock(&mut pointer, None, None).map_err(|err| os_error("lock input", err))?;
            std::ptr::copy_nonoverlapping(data.as_ptr(), pointer, data.len());
            buffer.Unlock().map_err(|err| os_error("unlock input", err))?;
            buffer.SetCurrentLength(data.len() as u32).map_err(|err| os_error("input length", err))?;
            let sample = MFCreateSample().map_err(|err| os_error("input sample", err))?;
            sample.AddBuffer(&buffer).map_err(|err| os_error("attach input", err))?;
            sample.SetSampleTime(self.sample_time).map_err(|err| os_error("sample time", err))?;
            self.sample_time += 1;
            Ok(sample)
        }
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Option<IMFSample>, PlatformError> {
        let sample = self.input_sample(data)?;
        unsafe { self.transform.ProcessInput(0, &sample, 0) }.map_err(|err| os_error("ProcessInput", err))?;
        let mut latest = None;
        loop {
            let mut output = [MFT_OUTPUT_DATA_BUFFER::default()];
            let mut status = 0u32;
            let result = unsafe { self.transform.ProcessOutput(0, &mut output, &mut status) };
            let produced = unsafe { std::mem::ManuallyDrop::take(&mut output[0].pSample) };
            drop(unsafe { std::mem::ManuallyDrop::take(&mut output[0].pEvents) });
            match result {
                Ok(()) => latest = produced.or(latest),
                Err(err) if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => return Ok(latest),
                Err(err) if err.code() == MF_E_TRANSFORM_STREAM_CHANGE => self.select_output()?,
                Err(err) => return Err(os_error("ProcessOutput", err)),
            }
        }
    }
}
