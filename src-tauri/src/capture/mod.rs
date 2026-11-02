pub mod ping;
pub mod vitals;

#[cfg(windows)]
pub mod pdh;

#[cfg(windows)]
pub mod disk_perf;

#[cfg(windows)]
pub mod gpu_perf;

#[cfg(windows)]
pub mod bt_battery;

#[cfg(windows)]
pub mod apple_battery;

#[cfg(windows)]
pub mod gatt_battery;

#[cfg(windows)]
pub mod media;

#[cfg(windows)]
pub mod lightspeed;
