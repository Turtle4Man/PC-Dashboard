use super::pdh::PdhSampler;

const DISK_RESPONSE_COUNTER: &str = "\\PhysicalDisk(*)\\Avg. Disk sec/Transfer";

pub struct DiskPerfSampler {
    sampler: PdhSampler,
}

impl DiskPerfSampler {
    pub fn new() -> Option<Self> {
        Some(Self {
            sampler: PdhSampler::new(DISK_RESPONSE_COUNTER)?,
        })
    }

    pub fn sample(&mut self) -> Vec<(String, f64)> {
        self.sampler
            .sample()
            .into_iter()
            .map(|(name, value)| (clean_disk_name(&name), value))
            .collect()
    }
}

fn clean_disk_name(name: &str) -> String {
    let trimmed = name.trim();
    let digits = trimmed.len()
        - trimmed
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .len();
    if digits > 0 {
        if let Some(rest) = trimmed.get(digits..) {
            let rest = rest.trim_start();
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_physical_disk_index() {
        assert_eq!(clean_disk_name("0 C:"), "C:");
        assert_eq!(clean_disk_name("1 D: E:"), "D: E:");
        assert_eq!(clean_disk_name("12 J:"), "J:");
    }

    #[test]
    fn keeps_total_and_plain_names() {
        assert_eq!(clean_disk_name("_Total"), "_Total");
        assert_eq!(clean_disk_name("C:"), "C:");
        assert_eq!(clean_disk_name(""), "");
    }
}
