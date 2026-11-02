use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct VitalsPayload {
    pub ts: u64,
    pub host: HostInfo,
    pub cpu: CpuInfo,
    pub mem: MemInfo,
    pub disks: Vec<DiskInfo>,
    pub gpus: Vec<GpuInfo>,
    pub bt: Vec<BtInfo>,
    pub net: Vec<NetInfo>,
    pub net_ping: Vec<NetPing>,
    pub mic: MicInfo,
    pub media: MediaInfo,
}

#[derive(Serialize, Clone)]
pub struct MediaInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    // Three-state thumbnail: absent = unchanged, null = cleared,
    // string = new art (data URL). Lets the frontend cache it instead of
    // receiving the bytes on every sample.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<Option<String>>,
}

#[derive(Serialize, Clone)]
pub struct NetPing {
    pub name: String,
    pub ms: f64,
}

#[derive(Serialize, Clone)]
pub struct MicInfo {
    pub muted: bool,
}

#[derive(Serialize, Clone)]
pub struct HostInfo {
    pub name: String,
    pub os: String,
    pub kernel: String,
}

#[derive(Serialize, Clone)]
pub struct CpuInfo {
    pub global: f32,
}

#[derive(Serialize, Clone)]
pub struct MemInfo {
    pub used: u64,
    pub total: u64,
}

#[derive(Serialize, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub resp_ms: f64,
}

#[derive(Serialize, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub usage: f64,
}

#[derive(Serialize, Clone)]
pub struct BtInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<BatteryState>,
}

#[derive(Serialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BatteryState {
    High,
    Low,
}

#[derive(Serialize, Clone)]
pub struct NetInfo {
    pub name: String,
    pub rx_bps: u64,
    pub tx_bps: u64,
    pub rx_total: u64,
    pub tx_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(thumb: Option<Option<String>>) -> MediaInfo {
        MediaInfo {
            playing: Some(true),
            title: Some("Classic Pokemon Lofi".to_string()),
            artist: None,
            thumb,
        }
    }

    fn payload(media: MediaInfo) -> VitalsPayload {
        VitalsPayload {
            ts: 42,
            host: HostInfo {
                name: "PC".to_string(),
                os: "Windows 11".to_string(),
                kernel: "10.0".to_string(),
            },
            cpu: CpuInfo { global: 12.5 },
            mem: MemInfo { used: 8, total: 16 },
            disks: vec![DiskInfo {
                name: "C:".to_string(),
                resp_ms: 0.4,
            }],
            gpus: vec![GpuInfo {
                name: "GPU".to_string(),
                usage: 3.0,
            }],
            bt: vec![BtInfo {
                name: "G305".to_string(),
                level: Some(88),
                state: Some(BatteryState::High),
            }],
            net: vec![NetInfo {
                name: "Ethernet".to_string(),
                rx_bps: 1,
                tx_bps: 2,
                rx_total: 3,
                tx_total: 4,
            }],
            net_ping: vec![NetPing {
                name: "Internet".to_string(),
                ms: 6.9,
            }],
            mic: MicInfo { muted: true },
            media,
        }
    }

    #[test]
    fn serializes_the_fields_the_interface_reads() {
        let value = serde_json::to_value(payload(media(None))).unwrap();

        assert_eq!(value["ts"], 42);
        assert_eq!(value["host"]["kernel"], "10.0");
        assert_eq!(value["cpu"]["global"], 12.5);
        assert_eq!(value["mem"]["total"], 16);
        assert_eq!(value["disks"][0]["name"], "C:");
        assert_eq!(value["disks"][0]["resp_ms"], 0.4);
        assert_eq!(value["gpus"][0]["usage"], 3.0);
        assert_eq!(value["net"][0]["rx_bps"], 1);
        assert_eq!(value["net"][0]["tx_total"], 4);
        assert_eq!(value["net_ping"][0]["ms"], 6.9);
        assert_eq!(value["mic"]["muted"], true);
        assert_eq!(value["bt"][0]["level"], 88);
        assert_eq!(value["bt"][0]["state"], "high");
    }

    #[test]
    fn thumbnail_keeps_its_three_states() {
        // Absent: keep the artwork the interface already caches.
        let absent = serde_json::to_value(payload(media(None))).unwrap();
        assert!(!absent["media"].as_object().unwrap().contains_key("thumb"));

        // Null: the track lost its artwork.
        let cleared = serde_json::to_value(payload(media(Some(None)))).unwrap();
        assert!(cleared["media"].as_object().unwrap().contains_key("thumb"));
        assert!(cleared["media"]["thumb"].is_null());

        // String: new artwork.
        let art = "data:image/png;base64,AAA";
        let fresh = serde_json::to_value(payload(media(Some(Some(art.to_string()))))).unwrap();
        assert_eq!(fresh["media"]["thumb"], art);
    }

    #[test]
    fn omits_unknown_media_and_device_fields() {
        let value = serde_json::to_value(payload(MediaInfo {
            playing: None,
            title: None,
            artist: None,
            thumb: None,
        }))
        .unwrap();
        assert!(value["media"].as_object().unwrap().is_empty());

        let device = serde_json::to_value(BtInfo {
            name: "Casque".to_string(),
            level: None,
            state: None,
        })
        .unwrap();
        assert_eq!(device.as_object().unwrap().len(), 1);
        assert_eq!(device["name"], "Casque");
    }

    #[test]
    fn serializes_battery_states_in_lowercase() {
        assert_eq!(
            serde_json::to_value(BatteryState::Low).unwrap(),
            serde_json::json!("low")
        );
        assert_eq!(
            serde_json::to_value(BatteryState::High).unwrap(),
            serde_json::json!("high")
        );
    }
}
