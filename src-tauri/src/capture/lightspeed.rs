use std::ffi::CString;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

use crate::events::BatteryState;

const LOGITECH_VID: u16 = 0x046D;
const HIDPP_USAGE_PAGE: u16 = 0xFF00;
const HIDPP_USAGE: u16 = 0x0002;

const REPORT_LONG_ID: u8 = 0x11;
const SOFTWARE_ID: u8 = 0x01;
const ROOT_FEATURE: u8 = 0x00;
const ROOT_GET_FEATURE: u8 = 0x00;
const ROOT_PING: u8 = 0x01;

const FEATURE_DEVICE_NAME: u16 = 0x0005;
const FEATURE_BATTERY_STATUS: u16 = 0x1000;
const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;

const FIRST_DEVICE_INDEX: u8 = 1;
const LAST_DEVICE_INDEX: u8 = 6;

const DISCOVERY_REFRESH: Duration = Duration::from_secs(30);
const RETRY_AFTER: Duration = Duration::from_secs(5);
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(200);

// HID++ 2.0 coarse levels: 0-10 critical, 11-30 low, 31-80 good, 81-100 full.
// Windows reports 30 as "good", so the low cutoff sits at 30.
const LOW_BATTERY_THRESHOLD: u8 = 30;

pub struct LightspeedDevice {
    pub name: String,
    pub level: Option<u8>,
    pub state: BatteryState,
}

pub struct LightspeedSampler {
    path: Option<CString>,
    receiver_name: Option<String>,
    devices: Vec<(u8, String)>,
    next_discovery: Option<Instant>,
}

impl LightspeedSampler {
    pub fn new() -> Self {
        Self {
            path: None,
            receiver_name: None,
            devices: Vec::new(),
            next_discovery: None,
        }
    }

    /// Returns the receiver's own product name and the names of the devices
    /// paired to it, discovering them on first use.
    pub fn paired_devices(&mut self) -> (Option<String>, Vec<String>) {
        let due = self.next_discovery.map_or(true, |at| Instant::now() >= at);
        if self.devices.is_empty() || due {
            self.discover();
        }
        let names = self.devices.iter().map(|(_, name)| name.clone()).collect();
        (self.receiver_name.clone(), names)
    }

    pub fn sample(&mut self) -> Vec<LightspeedDevice> {
        let due = self.next_discovery.map_or(true, |at| Instant::now() >= at);
        if self.devices.is_empty() || due {
            self.discover();
        }

        let Some(path) = self.path.clone() else {
            return Vec::new();
        };
        let Ok(api) = HidApi::new() else {
            self.reset(RETRY_AFTER);
            return Vec::new();
        };
        let Ok(device) = api.open_path(&path) else {
            self.reset(RETRY_AFTER);
            return Vec::new();
        };

        let mut result = Vec::new();
        for (index, name) in &self.devices {
            if let Some((level, state)) = read_battery(&device, *index) {
                result.push(LightspeedDevice {
                    name: name.clone(),
                    level,
                    state,
                });
            }
        }
        // Every cached device went silent (powered off, receiver moved): probe
        // again soon instead of polling a stale slot.
        if result.is_empty() {
            self.reset(RETRY_AFTER);
        }
        result
    }

    fn reset(&mut self, delay: Duration) {
        self.path = None;
        self.receiver_name = None;
        self.devices.clear();
        self.next_discovery = Some(Instant::now() + delay);
    }

    fn discover(&mut self) {
        self.path = None;
        self.receiver_name = None;
        self.devices.clear();
        self.next_discovery = Some(Instant::now() + DISCOVERY_REFRESH);

        let Ok(api) = HidApi::new() else {
            return;
        };
        let Some(info) = api.device_list().find(|info| {
            info.vendor_id() == LOGITECH_VID
                && info.usage_page() == HIDPP_USAGE_PAGE
                && info.usage() == HIDPP_USAGE
        }) else {
            return;
        };
        let path = info.path().to_owned();
        let receiver_name = info.product_string().map(str::to_string);
        let Ok(device) = api.open_path(&path) else {
            return;
        };

        let mut devices = Vec::new();
        for index in FIRST_DEVICE_INDEX..=LAST_DEVICE_INDEX {
            if transaction(&device, index, ROOT_FEATURE, ROOT_PING, &[0, 0, 0]).is_none() {
                continue;
            }
            let name = device_name(&device, index).unwrap_or_else(|| format!("Logitech {index}"));
            devices.push((index, name));
        }
        if devices.is_empty() {
            return;
        }
        self.path = Some(path);
        self.receiver_name = receiver_name;
        self.devices = devices;
    }
}

fn transaction(
    device: &HidDevice,
    index: u8,
    feature: u8,
    function: u8,
    params: &[u8],
) -> Option<Vec<u8>> {
    let mut report = [0u8; 20];
    report[0] = REPORT_LONG_ID;
    report[1] = index;
    report[2] = feature;
    report[3] = (function << 4) | SOFTWARE_ID;
    for (slot, value) in report[4..].iter_mut().zip(params.iter()) {
        *slot = *value;
    }
    device.write(&report).ok()?;

    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    let mut buffer = [0u8; 64];
    loop {
        match device.read_timeout(&mut buffer, 60) {
            Ok(size) if size >= 4 => {
                let response = &buffer[..size];
                if response[0] == REPORT_LONG_ID
                    && response[1] == index
                    && response[2] == feature
                    && response[3] == (function << 4) | SOFTWARE_ID
                {
                    return Some(response.to_vec());
                }
            }
            Ok(_) => {}
            Err(_) => return None,
        }
        if Instant::now() >= deadline {
            return None;
        }
    }
}

fn feature_index(device: &HidDevice, index: u8, feature_id: u16) -> Option<u8> {
    let params = [(feature_id >> 8) as u8, (feature_id & 0xFF) as u8, 0];
    let response = transaction(device, index, ROOT_FEATURE, ROOT_GET_FEATURE, &params)?;
    let feature = response[4];
    (feature != 0).then_some(feature)
}

fn device_name(device: &HidDevice, index: u8) -> Option<String> {
    let feature = feature_index(device, index, FEATURE_DEVICE_NAME)?;
    let count = transaction(device, index, feature, 0x00, &[0, 0, 0])?;
    let length = count[4] as usize;
    if length == 0 {
        return None;
    }

    let mut bytes = Vec::with_capacity(length);
    while bytes.len() < length {
        let offset = bytes.len() as u8;
        let chunk = transaction(device, index, feature, 0x01, &[offset, 0, 0])?;
        let take = (length - bytes.len()).min(16);
        bytes.extend_from_slice(&chunk[4..4 + take]);
    }

    let name = String::from_utf8_lossy(&bytes)
        .trim_end_matches('\0')
        .trim()
        .replace('_', " ");
    (!name.is_empty()).then_some(name)
}

fn read_battery(device: &HidDevice, index: u8) -> Option<(Option<u8>, BatteryState)> {
    if let Some(feature) = feature_index(device, index, FEATURE_UNIFIED_BATTERY) {
        let capabilities = transaction(device, index, feature, 0x00, &[0, 0, 0])?;
        let has_percentage = capabilities[5] & 0x02 != 0;
        let info = transaction(device, index, feature, 0x01, &[0, 0, 0])?;
        let percentage = info[4];
        let state = unified_state(info[5]);
        let level = (has_percentage && percentage > 0).then_some(percentage);
        return Some((level, state));
    }

    let feature = feature_index(device, index, FEATURE_BATTERY_STATUS)?;
    let capability = transaction(device, index, feature, 0x01, &[0, 0, 0]);
    let info = transaction(device, index, feature, 0x00, &[0, 0, 0])?;
    let capacity = info[4];
    let status = info[6];
    let mileage = capability
        .as_ref()
        .map(|reply| reports_mileage(reply[4], reply[5]))
        .unwrap_or(false);
    let level = (mileage && status == 0).then_some(capacity);
    Some((level, legacy_state(capacity, status)))
}

fn unified_state(level: u8) -> BatteryState {
    match level {
        // Critical / Low
        1 | 2 => BatteryState::Low,
        _ => BatteryState::High,
    }
}

fn legacy_state(capacity: u8, status: u8) -> BatteryState {
    match status {
        // Charge complete or charging.
        1..=4 => BatteryState::High,
        _ if capacity < LOW_BATTERY_THRESHOLD => BatteryState::Low,
        _ => BatteryState::High,
    }
}

fn reports_mileage(level_count: u8, flags: u8) -> bool {
    level_count >= 10 && flags & 0x02 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_levels_map_to_two_states() {
        assert!(matches!(unified_state(1), BatteryState::Low));
        assert!(matches!(unified_state(2), BatteryState::Low));
        assert!(matches!(unified_state(4), BatteryState::High));
        assert!(matches!(unified_state(8), BatteryState::High));
    }

    #[test]
    fn legacy_coarse_levels_split_at_the_windows_threshold() {
        assert!(matches!(legacy_state(20, 0), BatteryState::Low));
        assert!(matches!(legacy_state(29, 0), BatteryState::Low));
        assert!(matches!(legacy_state(30, 0), BatteryState::High));
        assert!(matches!(legacy_state(50, 0), BatteryState::High));
        assert!(matches!(legacy_state(100, 0), BatteryState::High));
    }

    #[test]
    fn charging_states_are_high_regardless_of_level() {
        for status in 1..=4 {
            assert!(matches!(legacy_state(5, status), BatteryState::High));
        }
    }

    #[test]
    fn mileage_needs_the_flag_and_ten_levels() {
        assert!(reports_mileage(100, 0x02));
        assert!(reports_mileage(10, 0x02));
        assert!(!reports_mileage(9, 0x02));
        assert!(!reports_mileage(100, 0x00));
    }
}
