use std::collections::HashMap;

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
};

use super::pdh::PdhSampler;

const GPU_ENGINE_COUNTER: &str = "\\GPU Engine(*)\\Utilization Percentage";

struct Adapter {
    luid: u64,
    name: String,
}

pub struct GpuPerfSampler {
    sampler: PdhSampler,
    adapters: Vec<Adapter>,
}

impl GpuPerfSampler {
    pub fn new() -> Option<Self> {
        Some(Self {
            sampler: PdhSampler::new(GPU_ENGINE_COUNTER)?,
            adapters: enumerate_adapters(),
        })
    }

    pub fn sample(&mut self) -> Vec<(String, f64)> {
        let mut sums: HashMap<u64, f64> = HashMap::new();
        for (instance, value) in self.sampler.sample() {
            if let Some(luid) = parse_luid(&instance) {
                *sums.entry(luid).or_insert(0.0) += value;
            }
        }

        self.adapters
            .iter()
            .map(|adapter| {
                let usage = sums
                    .get(&adapter.luid)
                    .copied()
                    .unwrap_or(0.0)
                    .clamp(0.0, 100.0);
                (adapter.name.clone(), usage)
            })
            .collect()
    }
}

pub fn adapter_names() -> Vec<String> {
    enumerate_adapters()
        .into_iter()
        .map(|adapter| adapter.name)
        .collect()
}

fn enumerate_adapters() -> Vec<Adapter> {
    let mut adapters = Vec::new();

    unsafe {
        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(factory) => factory,
            Err(_) => return adapters,
        };

        let mut index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            if let Ok(desc) = adapter.GetDesc1() {
                if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 == 0 {
                    let name = String::from_utf16_lossy(&desc.Description)
                        .trim_end_matches('\0')
                        .to_string();
                    let luid = ((desc.AdapterLuid.HighPart as u32 as u64) << 32)
                        | desc.AdapterLuid.LowPart as u64;
                    adapters.push(Adapter { luid, name });
                }
            }
            index += 1;
        }
    }

    adapters
}

fn parse_luid(instance: &str) -> Option<u64> {
    let tokens: Vec<&str> = instance.split('_').collect();
    let pos = tokens.iter().position(|token| *token == "luid")?;
    let high = parse_hex(tokens.get(pos + 1)?)?;
    let low = parse_hex(tokens.get(pos + 2)?)?;
    Some(((high as u64) << 32) | low as u64)
}

fn parse_hex(token: &str) -> Option<u32> {
    u32::from_str_radix(token.trim_start_matches("0x"), 16).ok()
}
