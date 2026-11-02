use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::DataReader;
use windows::Win32::System::Com::{
    COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
};
use windows::core::Ref;

// Layout reverse-engineered by the community (airinfo crate, OpenPods):
// Apple company id 0x004C, message type 0x07 ("proximity pairing"), 27 bytes.
// Battery levels live in plaintext nibbles of bytes 5-7 on a 0-10 scale
// (0x0F = unit not reporting); only the tail is encrypted.
const APPLE_COMPANY_ID: u16 = 0x004C;
const PROXIMITY_TYPE: u8 = 0x07;
const PROXIMITY_LEN: usize = 27;
const UNKNOWN_NIBBLE: u8 = 0x0F;
const FULL_SCALE: u8 = 10;

// Readings older than this are dropped; advertisements arrive every few
// seconds while the buds are active.
const STALE_AFTER: Duration = Duration::from_secs(120);

pub struct BudLevels {
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub case: Option<u8>,
}

pub struct AppleReading {
    pub addr: u64,
    pub level: u8,
    pub rssi: i16,
}

struct RawReading {
    left: Option<u8>,
    right: Option<u8>,
    #[allow(dead_code)]
    case: Option<u8>,
    rssi: i16,
    last_seen: Instant,
}

pub fn decode_proximity(frame: &[u8]) -> Option<BudLevels> {
    if frame.len() != PROXIMITY_LEN
        || frame[0] != PROXIMITY_TYPE
        || frame[1] as usize != PROXIMITY_LEN - 2
    {
        return None;
    }
    let nibble = |index: usize| -> u8 {
        let byte = frame[index / 2];
        if index % 2 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        }
    };
    let level = |value: u8| -> Option<u8> {
        if value == UNKNOWN_NIBBLE {
            None
        } else {
            Some(value.min(FULL_SCALE) * 10)
        }
    };
    // Nibbles 13/12: pod batteries, 15: case, 14: charge flags, 10: swap flag.
    let pod1 = level(nibble(13));
    let pod2 = level(nibble(12));
    let case = level(nibble(15));
    let swapped = nibble(10) & 0x02 == 0;
    let (left, right) = if swapped { (pod2, pod1) } else { (pod1, pod2) };
    Some(BudLevels { left, right, case })
}

fn bud_level(reading: &RawReading) -> Option<u8> {
    // Representative single value: the weakest reporting bud. The case is a
    // charger, not the listening device, so it never stands in for the buds.
    [reading.left, reading.right].into_iter().flatten().min()
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

pub struct AppleBatterySampler {
    _watcher: Option<BluetoothLEAdvertisementWatcher>,
    _com: ComGuard,
    readings: Arc<Mutex<HashMap<u64, RawReading>>>,
}

impl AppleBatterySampler {
    pub fn new() -> Self {
        let com = ComGuard::init();
        let readings: Arc<Mutex<HashMap<u64, RawReading>>> = Arc::new(Mutex::new(HashMap::new()));
        let watcher = start_watcher(Arc::clone(&readings));
        Self {
            _watcher: watcher,
            _com: com,
            readings,
        }
    }

    pub fn sample(&self) -> Vec<AppleReading> {
        let Ok(mut map) = self.readings.lock() else {
            return Vec::new();
        };
        map.retain(|_, reading| reading.last_seen.elapsed() < STALE_AFTER);
        map.iter()
            .filter_map(|(&addr, reading)| {
                bud_level(reading).map(|level| AppleReading {
                    addr,
                    level,
                    rssi: reading.rssi,
                })
            })
            .collect()
    }
}

impl Drop for AppleBatterySampler {
    fn drop(&mut self) {
        if let Some(watcher) = &self._watcher {
            let _ = watcher.Stop();
        }
    }
}

fn start_watcher(
    state: Arc<Mutex<HashMap<u64, RawReading>>>,
) -> Option<BluetoothLEAdvertisementWatcher> {
    let watcher = BluetoothLEAdvertisementWatcher::new().ok()?;
    let handler = TypedEventHandler::new(
        move |_sender: Ref<BluetoothLEAdvertisementWatcher>,
              args: Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
            let Some(args) = args.as_ref() else {
                return Ok(());
            };
            let addr = args.BluetoothAddress().unwrap_or(0);
            let rssi = args.RawSignalStrengthInDBm().unwrap_or(0);
            let advertisement = args.Advertisement()?;
            let items = advertisement.ManufacturerData()?;
            for index in 0..items.Size().unwrap_or(0) {
                let item = items.GetAt(index)?;
                if item.CompanyId().unwrap_or(0) != APPLE_COMPANY_ID {
                    continue;
                }
                let buffer = item.Data()?;
                let len = buffer.Length().unwrap_or(0) as usize;
                if len != PROXIMITY_LEN {
                    continue;
                }
                let reader = DataReader::FromBuffer(&buffer)?;
                let mut bytes = vec![0u8; len];
                reader.ReadBytes(&mut bytes)?;
                if let Some(levels) = decode_proximity(&bytes) {
                    if let Ok(mut map) = state.lock() {
                        map.insert(
                            addr,
                            RawReading {
                                left: levels.left,
                                right: levels.right,
                                case: levels.case,
                                rssi,
                                last_seen: Instant::now(),
                            },
                        );
                    }
                }
            }
            Ok(())
        },
    );
    watcher.Received(&handler).ok()?;
    watcher.Start().ok()?;
    Some(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real capture from the user's AirPods: right bud at 90% in ear,
    // left bud and case not reporting (closed case).
    const CAPTURED: [u8; 27] = [
        0x07, 0x19, 0x01, 0x19, 0x20, 0x02, 0xF9, 0x8F, 0x10, 0x00, 0x04, 0x35, 0x34, 0x31, 0xA3,
        0x8F, 0x90, 0xA7, 0x64, 0xFB, 0x54, 0x67, 0x2C, 0xA3, 0x2D, 0x89, 0xA5,
    ];

    #[test]
    fn decodes_captured_frame() {
        let levels = decode_proximity(&CAPTURED).expect("decodable");
        assert_eq!(levels.left, None);
        assert_eq!(levels.right, Some(90));
        assert_eq!(levels.case, None);
    }

    #[test]
    fn decodes_unswapped_frame() {
        // Nibble 10 has bit 1 set: no swap. Nibbles: L=7, R=8, case=9.
        let mut frame = CAPTURED;
        frame[5] = 0x22;
        frame[6] = 0x87;
        frame[7] = 0x19;
        let levels = decode_proximity(&frame).expect("decodable");
        assert_eq!(levels.left, Some(70));
        assert_eq!(levels.right, Some(80));
        assert_eq!(levels.case, Some(90));
    }

    #[test]
    fn rejects_non_proximity_frames() {
        assert!(decode_proximity(&CAPTURED[..26]).is_none());
        let mut frame = CAPTURED;
        frame[0] = 0x10;
        assert!(decode_proximity(&frame).is_none());
    }

    #[test]
    fn representative_level_uses_weakest_bud() {
        let reading = RawReading {
            left: Some(80),
            right: Some(100),
            case: Some(20),
            rssi: -60,
            last_seen: Instant::now(),
        };
        assert_eq!(bud_level(&reading), Some(80));
        let empty = RawReading {
            left: None,
            right: None,
            case: Some(100),
            rssi: -60,
            last_seen: Instant::now(),
        };
        assert_eq!(bud_level(&empty), None);
    }
}
