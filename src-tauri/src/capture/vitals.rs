use std::time::{SystemTime, UNIX_EPOCH};

use sysinfo::{
    CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, MemoryRefreshKind, Networks, RefreshKind, System,
};

use crate::events::{
    BtInfo, CpuInfo, DiskInfo, GpuInfo, HostInfo, MediaInfo, MemInfo, MicInfo, NetInfo,
    VitalsPayload,
};

#[cfg(windows)]
use crate::capture::apple_battery::AppleBatterySampler;
#[cfg(windows)]
use crate::capture::bt_battery::{BtBatterySampler, BtDevice};
#[cfg(windows)]
use crate::capture::disk_perf::DiskPerfSampler;
#[cfg(windows)]
use crate::capture::gatt_battery::GattBatterySampler;
#[cfg(windows)]
use crate::capture::gpu_perf::GpuPerfSampler;
#[cfg(windows)]
use crate::capture::lightspeed::LightspeedSampler;
#[cfg(windows)]
use crate::capture::media::MediaSampler;
use crate::capture::ping::PingSampler;

pub struct VitalsSampler {
    sys: System,
    networks: Networks,
    ping: PingSampler,
    #[cfg(windows)]
    disk_perf: Option<DiskPerfSampler>,
    #[cfg(windows)]
    gpu_perf: Option<GpuPerfSampler>,
    #[cfg(windows)]
    bt_battery: Option<BtBatterySampler>,
    #[cfg(windows)]
    apple_battery: Option<AppleBatterySampler>,
    #[cfg(windows)]
    gatt_battery: Option<GattBatterySampler>,
    #[cfg(windows)]
    lightspeed: Option<LightspeedSampler>,
    #[cfg(windows)]
    media: Option<MediaSampler>,
}

impl VitalsSampler {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_usage();
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);

        Self {
            sys,
            networks: Networks::new_with_refreshed_list(),
            ping: PingSampler::new(),
            #[cfg(windows)]
            disk_perf: DiskPerfSampler::new(),
            #[cfg(windows)]
            gpu_perf: GpuPerfSampler::new(),
            #[cfg(windows)]
            bt_battery: BtBatterySampler::new(),
            #[cfg(windows)]
            apple_battery: Some(AppleBatterySampler::new()),
            #[cfg(windows)]
            gatt_battery: Some(GattBatterySampler::new()),
            #[cfg(windows)]
            lightspeed: Some(LightspeedSampler::new()),
            #[cfg(windows)]
            media: Some(MediaSampler::new()),
        }
    }

    pub fn sample(&mut self, interval_ms: u64) -> VitalsPayload {
        let interval = interval_ms.max(1);

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.networks.refresh(true);

        let cpu = CpuInfo {
            global: self.sys.global_cpu_usage(),
        };

        let mem = MemInfo {
            used: self.sys.used_memory(),
            total: self.sys.total_memory(),
        };

        #[cfg(windows)]
        let disks = self
            .disk_perf
            .as_mut()
            .map(|d| d.sample())
            .unwrap_or_default()
            .into_iter()
            .map(|(name, seconds)| DiskInfo {
                name,
                resp_ms: (seconds * 1000.0).max(0.0),
            })
            .collect();

        #[cfg(not(windows))]
        let disks: Vec<DiskInfo> = Vec::new();

        #[cfg(windows)]
        let gpus = self
            .gpu_perf
            .as_mut()
            .map(|g| g.sample())
            .unwrap_or_default()
            .into_iter()
            .map(|(name, usage)| GpuInfo { name, usage })
            .collect();

        #[cfg(not(windows))]
        let gpus: Vec<GpuInfo> = Vec::new();

        #[cfg(windows)]
        let mut bt_devices: Vec<BtDevice> = self
            .bt_battery
            .as_ref()
            .map(|b| b.sample())
            .unwrap_or_default();

        #[cfg(windows)]
        {
            // Apple buds report battery only through BLE proximity frames,
            // never through Windows properties. The advertisement address is
            // randomized, so match exactly when possible, else fall back to
            // the single Apple device missing a level (strongest signal wins).
            const APPLE_VID: u16 = 0x004C;
            let mut readings = self
                .apple_battery
                .as_ref()
                .map(|a| a.sample())
                .unwrap_or_default();
            readings.sort_by(|a, b| b.rssi.cmp(&a.rssi));
            for reading in &readings {
                let addr = format!("{:012X}", reading.addr & 0xFFFF_FFFF_FFFF);
                if let Some(target) = bt_devices
                    .iter_mut()
                    .find(|d| d.level.is_none() && d.addr == addr)
                {
                    target.level = Some(reading.level);
                }
            }
            if let Some(reading) = readings.into_iter().next() {
                if let Some(target) = bt_devices
                    .iter_mut()
                    .find(|d| d.level.is_none() && d.vid == APPLE_VID)
                {
                    target.level = Some(reading.level);
                }
            }
        }

        #[cfg(windows)]
        let mut bt: Vec<BtInfo> = bt_devices
            .into_iter()
            .map(|d| BtInfo {
                name: d.name,
                level: d.level,
                state: None,
            })
            .collect();

        #[cfg(windows)]
        {
            let gatt = self
                .gatt_battery
                .as_mut()
                .map(|g| g.sample())
                .unwrap_or_default();
            for (name, level) in gatt {
                match bt.iter_mut().find(|b| b.name == name) {
                    Some(existing) => {
                        if existing.level.is_none() {
                            existing.level = Some(level);
                        }
                    }
                    None => bt.push(BtInfo {
                        name,
                        level: Some(level),
                        state: None,
                    }),
                }
            }
        }

        #[cfg(windows)]
        {
            // Wireless receivers (Lightspeed/Unifying) hide their paired
            // devices from the Bluetooth APIs. HID++ exposes them with a coarse
            // battery state instead of a percentage.
            let lightspeed = self
                .lightspeed
                .as_mut()
                .map(|l| l.sample())
                .unwrap_or_default();
            for device in lightspeed {
                match bt.iter_mut().find(|b| b.name == device.name) {
                    Some(existing) => {
                        if existing.level.is_none() {
                            existing.level = device.level;
                        }
                        if existing.state.is_none() {
                            existing.state = Some(device.state);
                        }
                    }
                    None => bt.push(BtInfo {
                        name: device.name,
                        level: device.level,
                        state: Some(device.state),
                    }),
                }
            }
        }

        #[cfg(not(windows))]
        let bt: Vec<BtInfo> = Vec::new();

        let net = self
            .networks
            .list()
            .iter()
            .map(|(name, data)| NetInfo {
                name: name.clone(),
                rx_bps: data.received().saturating_mul(1000) / interval,
                tx_bps: data.transmitted().saturating_mul(1000) / interval,
                rx_total: data.total_received(),
                tx_total: data.total_transmitted(),
            })
            .collect();

        let net_ping = self.ping.sample();

        #[cfg(windows)]
        let media = self.media.as_mut().and_then(|m| m.sample());

        #[cfg(windows)]
        let media_info = MediaInfo {
            playing: media.as_ref().map(|m| m.playing),
            title: media
                .as_ref()
                .map(|m| m.title.clone())
                .filter(|t| !t.trim().is_empty()),
            artist: media
                .as_ref()
                .map(|m| m.artist.clone())
                .filter(|t| !t.trim().is_empty()),
            thumb: media.as_ref().and_then(|m| m.thumb.clone()),
        };

        #[cfg(not(windows))]
        let media_info = MediaInfo {
            playing: None,
            title: None,
            artist: None,
        };

        let host = HostInfo {
            name: System::host_name().unwrap_or_default(),
            os: format!(
                "{} {}",
                System::name().unwrap_or_default(),
                System::os_version().unwrap_or_default()
            )
            .trim()
            .to_string(),
            kernel: System::kernel_version().unwrap_or_default(),
        };

        VitalsPayload {
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            host,
            cpu,
            mem,
            disks,
            gpus,
            bt,
            net,
            net_ping,
            mic: MicInfo { muted: false },
            media: media_info,
        }
    }
}
