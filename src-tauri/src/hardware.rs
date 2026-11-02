use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use hidapi::HidApi;
use serde::Serialize;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

use crate::capture::bt_battery::{device_class, friendly_name, locate_device, present_device_ids};
use crate::capture::lightspeed::LightspeedSampler;

const DISK_DRIVE_CLASS: &str = "DiskDrive";
const AUDIO_ENDPOINT_CLASS: &str = "AudioEndpoint";
const MONITOR_CLASS: &str = "Monitor";

const USAGE_PAGE_GENERIC_DESKTOP: u16 = 0x01;
const USAGE_MOUSE: u16 = 0x02;
const USAGE_JOYSTICK: u16 = 0x04;
const USAGE_GAME_PAD: u16 = 0x05;
const USAGE_KEYBOARD: u16 = 0x06;
const USAGE_MULTI_AXIS: u16 = 0x08;

const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;
const BIOS_KEY: &str = r"HARDWARE\DESCRIPTION\System\BIOS";

#[derive(Serialize, Clone)]
pub struct Component {
    pub label: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PeripheralGroup {
    pub label: String,
    pub items: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct HardwarePayload {
    pub components: Vec<Component>,
    pub peripherals: Vec<PeripheralGroup>,
}

pub fn collect() -> HardwarePayload {
    let devices = present_devices();
    HardwarePayload {
        components: components(&devices),
        peripherals: peripherals(&devices),
    }
}

fn components(devices: &[(String, String)]) -> Vec<Component> {
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    let mut components = Vec::new();

    if let Some(cpu) = sys.cpus().first() {
        let brand = cpu.brand().trim();
        if !brand.is_empty() {
            components.push(Component {
                label: "Processeur".to_string(),
                name: brand.to_string(),
                detail: core_detail(System::physical_core_count(), sys.cpus().len()),
            });
        }
    }

    let mut seen_gpus = HashSet::new();
    for name in crate::capture::gpu_perf::adapter_names() {
        if seen_gpus.insert(name.to_lowercase()) {
            components.push(Component {
                label: "Carte graphique".to_string(),
                name,
                detail: None,
            });
        }
    }

    if sys.total_memory() > 0 {
        components.push(Component {
            label: "Mémoire".to_string(),
            name: format_gb(sys.total_memory()),
            detail: None,
        });
    }

    if let Some(board) = motherboard() {
        components.push(Component {
            label: "Carte mère".to_string(),
            name: board,
            detail: None,
        });
    }

    for name in class_device_names(devices, DISK_DRIVE_CLASS) {
        components.push(Component {
            label: "Stockage".to_string(),
            name,
            detail: None,
        });
    }

    if let Some(os) = System::long_os_version() {
        components.push(Component {
            label: "Système".to_string(),
            name: os,
            detail: System::kernel_version().filter(|version| !version.trim().is_empty()),
        });
    }

    components
}

fn peripherals(devices: &[(String, String)]) -> Vec<PeripheralGroup> {
    let input = input_devices();
    let audio = class_device_names(devices, AUDIO_ENDPOINT_CLASS);
    let monitors = class_device_names(devices, MONITOR_CLASS);

    [("Entrée", input), ("Audio", audio), ("Écrans", monitors)]
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(label, items)| PeripheralGroup {
            label: label.to_string(),
            items,
        })
        .collect()
}

fn input_devices() -> Vec<String> {
    let mut devices = Vec::new();

    if let Ok(api) = HidApi::new() {
        for device in api.device_list() {
            let Some(name) = device
                .product_string()
                .map(str::trim)
                .filter(|name| !name.is_empty())
            else {
                continue;
            };

            if device.usage_page() != USAGE_PAGE_GENERIC_DESKTOP {
                continue;
            }

            if matches!(
                device.usage(),
                USAGE_KEYBOARD | USAGE_MOUSE | USAGE_JOYSTICK | USAGE_GAME_PAD | USAGE_MULTI_AXIS
            ) {
                push_unique(&mut devices, name);
            }
        }
    }

    // A Lightspeed receiver exposes its own generic name ("USB Receiver")
    // through HID. Prefer the actual paired device names from HID++.
    let (receiver_name, paired) = lightspeed_devices();
    if !paired.is_empty() {
        if let Some(receiver_name) = receiver_name {
            devices.retain(|name| !name.eq_ignore_ascii_case(&receiver_name));
        }
        for name in paired {
            push_unique(&mut devices, &name);
        }
    }

    devices
}

fn push_unique(group: &mut Vec<String>, name: &str) {
    if !group
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(name))
    {
        group.push(name.to_string());
    }
}

fn lightspeed_devices() -> (Option<String>, Vec<String>) {
    static SAMPLER: OnceLock<Mutex<LightspeedSampler>> = OnceLock::new();
    let sampler = SAMPLER.get_or_init(|| Mutex::new(LightspeedSampler::new()));
    match sampler.lock() {
        Ok(mut sampler) => sampler.paired_devices(),
        Err(_) => (None, Vec::new()),
    }
}

fn present_devices() -> Vec<(String, String)> {
    let mut devices = Vec::new();

    for id in present_device_ids() {
        let Some(devinst) = locate_device(&id) else {
            continue;
        };
        let (Some(class), Some(name)) = (device_class(devinst), friendly_name(devinst)) else {
            continue;
        };
        let name = name.trim().to_string();
        if !name.is_empty() {
            devices.push((class, name));
        }
    }

    devices
}

fn class_device_names(devices: &[(String, String)], class: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut names: Vec<String> = devices
        .iter()
        .filter(|(device_class, _)| device_class.eq_ignore_ascii_case(class))
        .map(|(_, name)| name)
        .filter(|name| seen.insert(name.to_lowercase()))
        .cloned()
        .collect();

    names.sort_by_cached_key(|name| name.to_lowercase());
    names
}

fn motherboard() -> Option<String> {
    board_label(
        reg_string(BIOS_KEY, "BaseBoardManufacturer"),
        reg_string(BIOS_KEY, "BaseBoardProduct"),
    )
}

/// Joins the board information without repeating the manufacturer when the
/// product name already starts with it.
fn board_label(manufacturer: Option<String>, product: Option<String>) -> Option<String> {
    match (manufacturer, product) {
        (Some(manufacturer), Some(product)) if product.contains(&manufacturer) => Some(product),
        (Some(manufacturer), Some(product)) => Some(format!("{manufacturer} {product}")),
        (Some(manufacturer), None) => Some(manufacturer),
        (None, Some(product)) => Some(product),
        (None, None) => None,
    }
}

fn reg_string(subkey: &str, value: &str) -> Option<String> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(subkey)
        .ok()?;
    let text: String = key.get_value(value).ok()?;
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn core_detail(physical: Option<usize>, logical: usize) -> Option<String> {
    match (physical, logical) {
        (Some(physical), logical) if physical > 0 && logical > 0 && physical < logical => {
            Some(format!("{physical} cœurs · {logical} threads"))
        }
        (Some(physical), _) if physical > 0 => Some(format!("{physical} cœurs")),
        (_, logical) if logical > 0 => Some(format!("{logical} threads")),
        _ => None,
    }
}

fn format_gb(bytes: u64) -> String {
    let gb = bytes as f64 / BYTES_PER_GB;
    if gb >= 10.0 {
        format!("{gb:.0} Go")
    } else {
        format!("{gb:.1} Go")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn formats_memory_with_one_decimal_below_ten_gigabytes() {
        assert_eq!(format_gb(8 * GB), "8.0 Go");
        assert_eq!(format_gb(9 * GB + GB / 2), "9.5 Go");
    }

    #[test]
    fn formats_memory_without_decimals_from_ten_gigabytes() {
        assert_eq!(format_gb(10 * GB), "10 Go");
        assert_eq!(format_gb(32 * GB), "32 Go");
    }

    #[test]
    fn describes_cores_and_threads() {
        assert_eq!(
            core_detail(Some(6), 12).as_deref(),
            Some("6 cœurs · 12 threads")
        );
        assert_eq!(core_detail(Some(6), 6).as_deref(), Some("6 cœurs"));
        assert_eq!(core_detail(None, 12).as_deref(), Some("12 threads"));
        assert_eq!(core_detail(Some(0), 8).as_deref(), Some("8 threads"));
        assert_eq!(core_detail(None, 0), None);
    }

    #[test]
    fn avoids_repeating_the_board_manufacturer() {
        let label = |manufacturer: &str, product: &str| {
            board_label(Some(manufacturer.to_string()), Some(product.to_string()))
        };

        assert_eq!(
            label("ASUS", "ASUS PRIME B760").as_deref(),
            Some("ASUS PRIME B760")
        );
        assert_eq!(
            label("ASUS", "PRIME B760").as_deref(),
            Some("ASUS PRIME B760")
        );
    }

    #[test]
    fn keeps_whatever_board_information_exists() {
        assert_eq!(
            board_label(Some("MSI".to_string()), None).as_deref(),
            Some("MSI")
        );
        assert_eq!(
            board_label(None, Some("B650".to_string())).as_deref(),
            Some("B650")
        );
        assert_eq!(board_label(None, None), None);
    }

    #[test]
    fn lists_each_device_once_within_its_class() {
        let devices = vec![
            ("DiskDrive".to_string(), "Samsung SSD".to_string()),
            ("diskdrive".to_string(), "samsung ssd".to_string()),
            ("DiskDrive".to_string(), "WD Blue".to_string()),
            ("Monitor".to_string(), "Dell".to_string()),
        ];

        assert_eq!(
            class_device_names(&devices, "DiskDrive"),
            vec!["Samsung SSD", "WD Blue"]
        );
        assert_eq!(class_device_names(&devices, "Monitor"), vec!["Dell"]);
        assert!(class_device_names(&devices, "AudioEndpoint").is_empty());
    }

    #[test]
    fn pushes_input_names_once_regardless_of_case() {
        let mut names = vec!["USB Keyboard".to_string()];

        push_unique(&mut names, "usb keyboard");
        push_unique(&mut names, "G305");

        assert_eq!(names, vec!["USB Keyboard", "G305"]);
    }
}
