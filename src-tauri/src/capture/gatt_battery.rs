use std::future::IntoFuture;
use std::time::{Duration, Instant};
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristicProperties, GattCharacteristicUuids, GattCommunicationStatus,
    GattServiceUuids, GattSharingMode,
};
use windows::Devices::Enumeration::{DeviceAccessStatus, DeviceInformation};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::Com::{
    COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
};
use windows::core::HSTRING;
use windows_future::IAsyncOperation;

const REFRESH_SECS: u64 = 30;

fn block<T: windows::core::RuntimeType>(
    operation: IAsyncOperation<T>,
) -> Result<T, windows::core::Error> {
    pollster::block_on(operation.into_future())
}

struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    fn init() -> Self {
        let initialized =
            unsafe { CoInitializeEx(None, COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE).is_ok() };
        Self { initialized }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

pub struct GattBatterySampler {
    levels: Vec<(String, u8)>,
    last: Instant,
    _com: ComGuard,
}

impl GattBatterySampler {
    pub fn new() -> Self {
        let com = ComGuard::init();
        Self {
            levels: Vec::new(),
            last: Instant::now()
                .checked_sub(Duration::from_secs(REFRESH_SECS + 1))
                .unwrap_or(Instant::now()),
            _com: com,
        }
    }

    pub fn sample(&mut self) -> Vec<(String, u8)> {
        if self.last.elapsed().as_secs() < REFRESH_SECS {
            return self.levels.clone();
        }
        let levels = read_uwp_paired();
        self.levels = levels.clone();
        self.last = Instant::now();
        levels
    }
}

impl Default for GattBatterySampler {
    fn default() -> Self {
        Self::new()
    }
}

fn read_uwp_paired() -> Vec<(String, u8)> {
    let selector = match BluetoothLEDevice::GetDeviceSelectorFromPairingState(true) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let collection = match DeviceInformation::FindAllAsyncAqsFilter(&selector).and_then(block) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let size = collection.Size().unwrap_or(0);
    let battery_uuid = match GattServiceUuids::Battery() {
        Ok(u) => u,
        Err(_) => return Vec::new(),
    };
    let level_uuid = match GattCharacteristicUuids::BatteryLevel() {
        Ok(u) => u,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for index in 0..size {
        let info = match collection.GetAt(index) {
            Ok(i) => i,
            Err(_) => continue,
        };
        let name = match info.Name() {
            Ok(n) => n.to_string(),
            Err(_) => continue,
        };
        if name.trim().is_empty() {
            continue;
        }
        let id = match info.Id() {
            Ok(i) => i,
            Err(_) => continue,
        };
        for level in read_device_levels(&id, battery_uuid, level_uuid) {
            let entry = (name.clone(), level);
            if !out.contains(&entry) {
                out.push(entry);
            }
        }
    }
    out
}

fn read_device_levels(
    id: &HSTRING,
    battery_uuid: windows::core::GUID,
    level_uuid: windows::core::GUID,
) -> Vec<u8> {
    let Ok(device) = BluetoothLEDevice::FromIdAsync(id).and_then(block) else {
        return Vec::new();
    };
    let Ok(access) = device.RequestAccessAsync().and_then(block) else {
        return Vec::new();
    };
    if access != DeviceAccessStatus::Allowed {
        return Vec::new();
    }
    let Ok(services) = device.GetGattServicesAsync().and_then(block) else {
        return Vec::new();
    };
    if services
        .Status()
        .unwrap_or(GattCommunicationStatus::Unreachable)
        != GattCommunicationStatus::Success
    {
        return Vec::new();
    }
    let Ok(view) = services.Services() else {
        return Vec::new();
    };
    let count = view.Size().unwrap_or(0);
    let mut levels = Vec::new();
    for index in 0..count {
        let Ok(service) = view.GetAt(index) else {
            continue;
        };
        if service.Uuid().unwrap_or_default() != battery_uuid {
            continue;
        }
        service.OpenAsync(GattSharingMode::SharedReadOnly).ok();
        let Ok(chars) = service.GetCharacteristicsAsync().and_then(block) else {
            continue;
        };
        let Ok(chars_view) = chars.Characteristics() else {
            continue;
        };
        for cindex in 0..chars_view.Size().unwrap_or(0) {
            if let Ok(c) = chars_view.GetAt(cindex) {
                read_characteristic(&c, &mut levels, level_uuid);
            }
        }
    }
    levels
}

fn read_characteristic(
    characteristic: &windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristic,
    levels: &mut Vec<u8>,
    level_uuid: windows::core::GUID,
) {
    if characteristic.Uuid().unwrap_or_default() != level_uuid {
        return;
    }
    let properties = characteristic
        .CharacteristicProperties()
        .unwrap_or_default();
    if properties.0 & GattCharacteristicProperties::Read.0 == 0 {
        return;
    }
    let Ok(read) = characteristic.ReadValueAsync().and_then(block) else {
        return;
    };
    if read
        .Status()
        .unwrap_or(GattCommunicationStatus::Unreachable)
        != GattCommunicationStatus::Success
    {
        return;
    }
    let Ok(buffer) = read.Value() else {
        return;
    };
    let length = buffer.Length().unwrap_or(0) as usize;
    if length == 0 {
        return;
    }
    let Ok(reader) = DataReader::FromBuffer(&buffer) else {
        return;
    };
    let mut bytes = vec![0u8; length];
    if reader.ReadBytes(&mut bytes).is_err() {
        return;
    }
    if bytes[0] <= 100 {
        levels.push(bytes[0]);
    }
}
