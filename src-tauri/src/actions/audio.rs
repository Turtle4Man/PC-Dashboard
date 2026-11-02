use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eCapture, eConsole};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};

struct ComGuard(bool);

impl ComGuard {
    fn new() -> Self {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        ComGuard(hr.0 == 0)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

fn default_capture_volume() -> windows::core::Result<IAudioEndpointVolume> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;
        device.Activate(CLSCTX_ALL, None)
    }
}

pub fn is_muted() -> Option<bool> {
    let _com = ComGuard::new();
    let volume = default_capture_volume().ok()?;
    unsafe { volume.GetMute().ok() }.map(|b| b.as_bool())
}

pub fn set_muted(muted: bool) -> Result<bool, String> {
    let _com = ComGuard::new();
    let volume = default_capture_volume().map_err(|e| format!("audio endpoint: {e}"))?;
    unsafe {
        volume
            .SetMute(muted, std::ptr::null())
            .map_err(|e| format!("SetMute: {e}"))?;
        volume
            .GetMute()
            .map(|b| b.as_bool())
            .map_err(|e| format!("GetMute: {e}"))
    }
}
