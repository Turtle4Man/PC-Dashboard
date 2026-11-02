use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::future::IntoFuture;
use std::time::{Duration, Instant};

use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationCollection};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_GETIDLIST_FILTER_PRESENT, CM_Get_DevNode_PropertyW, CM_Get_Device_ID_List_SizeW,
    CM_Get_Device_ID_ListW, CM_LOCATE_DEVNODE_NORMAL, CM_Locate_DevNodeW, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{DEVPKEY_Device_FriendlyName, DEVPROPTYPE};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::Win32::System::Com::{
    COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
};
use windows::core::{GUID, HSTRING, PCWSTR};
use windows_future::IAsyncOperation;

const DEVPKEY_BLUETOOTH_BATTERY_LEVEL: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5),
    pid: 2,
};

const DEVPKEY_DEVICE_CLASS: DEVPROPKEY = DEVPROPKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 9,
};

const DEVPROP_TYPE_BYTE: u32 = 0x0000_0003;
const DEVPROP_TYPE_STRING: u32 = 0x0000_0012;

const CONNECTED_REFRESH: Duration = Duration::from_secs(3);

pub struct BtDevice {
    pub name: String,
    pub level: Option<u8>,
    pub addr: String,
    pub vid: u16,
}

pub struct BtBatterySampler {
    connected: RefCell<Option<(Instant, Option<HashSet<String>>)>>,
    _com: ComGuard,
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

fn block<T: windows::core::RuntimeType>(
    operation: IAsyncOperation<T>,
) -> Result<T, windows::core::Error> {
    pollster::block_on(operation.into_future())
}

impl BtBatterySampler {
    pub fn new() -> Option<Self> {
        Some(Self {
            connected: RefCell::new(None),
            _com: ComGuard::init(),
        })
    }

    fn connected_addresses(&self) -> Option<HashSet<String>> {
        let mut cache = self.connected.borrow_mut();
        if let Some((at, set)) = cache.as_ref() {
            if at.elapsed() < CONNECTED_REFRESH {
                return set.clone();
            }
        }
        let fresh = connected_bluetooth_addresses();
        *cache = Some((Instant::now(), fresh.clone()));
        fresh
    }

    pub fn sample(&self) -> Vec<BtDevice> {
        let connected = self.connected_addresses();
        let present = present_device_ids();
        let vids = vid_by_address(&present);
        let mut result = Vec::new();
        for id in &present {
            if !is_bluetooth_device(&id) {
                continue;
            }
            if let Some(set) = &connected {
                let matches = address_from_instance_id(&id)
                    .map(|addr| set.contains(&addr))
                    .unwrap_or(false);
                if !matches {
                    continue;
                }
            }
            let addr = address_from_instance_id(id).unwrap_or_default();
            let vid = vids.get(&addr).copied().unwrap_or(0);
            let Some(devinst) = locate_device(id) else {
                continue;
            };
            let Some(name) = friendly_name(devinst) else {
                continue;
            };
            let level = battery_level(devinst);
            result.push(BtDevice {
                name,
                level,
                addr,
                vid,
            });
        }
        result.dedup_by(|a, b| a.name == b.name);
        result
    }
}

fn connected_bluetooth_addresses() -> Option<HashSet<String>> {
    let mut set = HashSet::new();
    let le_selector = BluetoothLEDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    )
    .ok()?;
    collect_addresses(&le_selector, &mut set)?;
    let classic_selector = BluetoothDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    )
    .ok()?;
    collect_addresses(&classic_selector, &mut set)?;
    Some(set)
}

fn collect_addresses(selector: &HSTRING, set: &mut HashSet<String>) -> Option<()> {
    let collection: DeviceInformationCollection =
        DeviceInformation::FindAllAsyncAqsFilter(selector)
            .and_then(block)
            .ok()?;
    let size = collection.Size().ok()?;
    for index in 0..size {
        let Ok(info) = collection.GetAt(index) else {
            continue;
        };
        let Ok(id) = info.Id() else {
            continue;
        };
        if let Some(addr) = address_from_device_id(&id.to_string()) {
            set.insert(addr);
        }
    }
    Some(())
}

fn is_bluetooth_device(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    lower.starts_with("bthenum\\dev_") || lower.starts_with("bthle\\dev_")
}

fn address_from_instance_id(id: &str) -> Option<String> {
    let lower = id.to_ascii_lowercase();
    let start = lower.find("dev_")? + 4;
    let addr = id.get(start..start + 12)?;
    addr.chars()
        .all(|c| c.is_ascii_hexdigit())
        .then(|| addr.to_ascii_uppercase())
}

fn address_from_device_id(id: &str) -> Option<String> {
    let tail = id.rsplit('-').next()?;
    let addr: String = tail.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (addr.len() == 12).then(|| addr.to_ascii_uppercase())
}

fn vid_by_address(ids: &[String]) -> HashMap<String, u16> {
    // Service nodes carry the vendor id, e.g.
    // BTHENUM\{...}_VID&0001004C_PID&2019\...\1C775418A082_C00000000
    // where the last 4 hex digits of the VID field are the vendor id.
    let mut map = HashMap::new();
    for id in ids {
        let lower = id.to_ascii_lowercase();
        let Some(start) = lower.find("vid&") else {
            continue;
        };
        let Some(field) = id.get(start + 4..start + 12) else {
            continue;
        };
        let digits = &field[field.len().saturating_sub(4)..];
        let Ok(vid) = u16::from_str_radix(digits, 16) else {
            continue;
        };
        let Some(tail) = id.rsplit('\\').next() else {
            continue;
        };
        // The address sits in an &-separated field shaped like
        // <12 hex>_C00000000, e.g. 8&5802194&0&1C775418A082_C00000000.
        let addr = tail
            .split('&')
            .filter_map(|part| part.split('_').next())
            .find(|part| part.len() == 12 && part.chars().all(|c| c.is_ascii_hexdigit()));
        let Some(addr) = addr else {
            continue;
        };
        map.insert(addr.to_ascii_uppercase(), vid);
    }
    map
}

pub fn present_device_ids() -> Vec<String> {
    unsafe {
        let mut size: u32 = 0;
        if CM_Get_Device_ID_List_SizeW(&mut size, PCWSTR::null(), CM_GETIDLIST_FILTER_PRESENT).0
            != 0
        {
            return Vec::new();
        }

        let mut buffer = vec![0u16; size as usize];
        if CM_Get_Device_ID_ListW(PCWSTR::null(), &mut buffer, CM_GETIDLIST_FILTER_PRESENT).0 != 0 {
            return Vec::new();
        }

        let mut ids = Vec::new();
        let mut start = 0;
        for (index, &unit) in buffer.iter().enumerate() {
            if unit == 0 {
                if index == start {
                    break;
                }
                ids.push(String::from_utf16_lossy(&buffer[start..index]));
                start = index + 1;
            }
        }
        ids
    }
}

pub(crate) fn locate_device(id: &str) -> Option<u32> {
    unsafe {
        let wide: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();
        let mut devinst: u32 = 0;
        CM_Locate_DevNodeW(
            &mut devinst,
            PCWSTR(wide.as_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        );
        (devinst != 0).then_some(devinst)
    }
}

fn get_property(
    devinst: u32,
    key: &DEVPROPKEY,
    buffer: &mut [u8],
) -> Result<(DEVPROPTYPE, usize), ()> {
    unsafe {
        let mut prop_type = DEVPROPTYPE(0);
        let mut size = buffer.len() as u32;
        let ret = CM_Get_DevNode_PropertyW(
            devinst,
            key,
            &mut prop_type,
            Some(buffer.as_mut_ptr()),
            &mut size,
            0,
        );
        if ret != CR_SUCCESS {
            return Err(());
        }
        Ok((prop_type, size as usize))
    }
}

fn battery_level(devinst: u32) -> Option<u8> {
    let mut buffer = [0u8; 1];
    match get_property(devinst, &DEVPKEY_BLUETOOTH_BATTERY_LEVEL, &mut buffer) {
        Ok((prop_type, size)) if prop_type.0 == DEVPROP_TYPE_BYTE && size == 1 => Some(buffer[0]),
        _ => None,
    }
}

pub(crate) fn friendly_name(devinst: u32) -> Option<String> {
    string_property(devinst, &DEVPKEY_Device_FriendlyName)
}

pub(crate) fn device_class(devinst: u32) -> Option<String> {
    string_property(devinst, &DEVPKEY_DEVICE_CLASS)
}

fn string_property(devinst: u32, key: &DEVPROPKEY) -> Option<String> {
    let mut buffer = vec![0u8; 512];
    match get_property(devinst, key, &mut buffer) {
        Ok((prop_type, size)) if prop_type.0 == DEVPROP_TYPE_STRING => {
            let units = buffer[..size.min(buffer.len())]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .take_while(|&unit| unit != 0)
                .collect::<Vec<u16>>();
            let name = String::from_utf16_lossy(&units);
            (!name.is_empty()).then_some(name)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_instance_id_address() {
        assert_eq!(
            address_from_instance_id(
                "BTHENUM\\DEV_98B6EA8FE116\\8&5802194&0&BLUETOOTHDEVICE_98B6EA8FE116"
            ),
            Some("98B6EA8FE116".to_string())
        );
        assert_eq!(
            address_from_instance_id("BTHLE\\DEV_DD83AF1F4754\\8&AD6E33&0&DD83AF1F4754"),
            Some("DD83AF1F4754".to_string())
        );
    }

    #[test]
    fn parses_device_id_address() {
        assert_eq!(
            address_from_device_id("BluetoothLE#BluetoothLE20:e1:5d:2b:6d:1d-dd:83:af:1f:47:54"),
            Some("DD83AF1F4754".to_string())
        );
        assert_eq!(
            address_from_device_id("Bluetooth#Bluetooth20:e1:5d:2b:6d:1d-1c:77:54:18:a0:82"),
            Some("1C775418A082".to_string())
        );
    }

    #[test]
    fn parses_service_node_vids() {
        let ids = vec![
            "BTHENUM\\{00001801-0000-1000-8000-00805F9B34FB}_VID&0001004C_PID&2019\\8&5802194&0&1C775418A082_C00000000"
                .to_string(),
            "BTHENUM\\DEV_1C775418A082\\8&5802194&0&BLUETOOTHDEVICE_1C775418A082".to_string(),
        ];
        let map = vid_by_address(&ids);
        assert_eq!(map.get("1C775418A082"), Some(&0x004C));
        assert_eq!(map.len(), 1);
    }
}
