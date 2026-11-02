use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW,
};
use windows::core::PCWSTR;

pub struct PdhSampler {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl PdhSampler {
    pub fn new(path: &str) -> Option<Self> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }

            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

            let mut counter = PDH_HCOUNTER::default();
            if PdhAddEnglishCounterW(query, PCWSTR(wide.as_ptr()), 0, &mut counter) != 0 {
                let _ = PdhCloseQuery(query);
                return None;
            }

            let sampler = Self { query, counter };
            let _ = PdhCollectQueryData(sampler.query);
            Some(sampler)
        }
    }

    pub fn sample(&mut self) -> Vec<(String, f64)> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return Vec::new();
            }
            collect_array(self.counter)
        }
    }
}

impl Drop for PdhSampler {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}

unsafe fn collect_array(counter: PDH_HCOUNTER) -> Vec<(String, f64)> {
    let mut size: u32 = 0;
    let mut count: u32 = 0;

    let status = unsafe {
        PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None)
    };
    if status != PDH_MORE_DATA || count == 0 || size == 0 {
        return Vec::new();
    }

    let elem = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
    let slots = (size as usize).div_ceil(elem);
    let mut buffer = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); slots];

    let status = unsafe {
        PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut size,
            &mut count,
            Some(buffer.as_mut_ptr()),
        )
    };
    if status != 0 {
        return Vec::new();
    }

    buffer
        .iter()
        .take(count as usize)
        .map(|item| {
            let name = unsafe { item.szName.to_string() }.unwrap_or_default();
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            (name, value)
        })
        .collect()
}
