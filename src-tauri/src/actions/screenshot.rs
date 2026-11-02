use std::path::PathBuf;

use serde::Serialize;
use xcap::Monitor;
use xcap::image::{RgbaImage, imageops};

#[derive(Serialize)]
pub struct ScreenshotInfo {
    pub width: u32,
    pub height: u32,
    pub monitors: usize,
    pub path: String,
}

pub fn capture_to_file(all_screens: bool) -> Result<ScreenshotInfo, String> {
    let monitors = Monitor::all().map_err(|e| format!("Monitor::all: {e}"))?;
    if monitors.is_empty() {
        return Err("aucun écran détecté".into());
    }

    let selected: Vec<&Monitor> = if all_screens {
        monitors.iter().collect()
    } else {
        let primary = monitors
            .iter()
            .find(|monitor| monitor.is_primary().unwrap_or(false))
            .unwrap_or(&monitors[0]);
        vec![primary]
    };

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for monitor in &selected {
        let x = monitor.x().map_err(|e| e.to_string())?;
        let y = monitor.y().map_err(|e| e.to_string())?;
        let w = monitor.width().map_err(|e| e.to_string())? as i32;
        let h = monitor.height().map_err(|e| e.to_string())? as i32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let width = (max_x - min_x) as u32;
    let height = (max_y - min_y) as u32;
    let mut canvas = RgbaImage::new(width, height);

    for monitor in &selected {
        let shot = monitor
            .capture_image()
            .map_err(|e| format!("capture: {e}"))?;
        let x = monitor.x().map_err(|e| e.to_string())? - min_x;
        let y = monitor.y().map_err(|e| e.to_string())? - min_y;
        imageops::overlay(&mut canvas, &shot, x as i64, y as i64);
    }

    let dir = screenshots_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("dossier captures : {e}"))?;
    let stamp = timestamp();
    let mut path = dir.join(format!("capture-{stamp}.png"));
    let mut counter = 1;
    while path.exists() {
        counter += 1;
        path = dir.join(format!("capture-{stamp}-{counter}.png"));
    }
    canvas
        .save(&path)
        .map_err(|e| format!("enregistrement : {e}"))?;

    Ok(ScreenshotInfo {
        width,
        height,
        monitors: selected.len(),
        path: path.to_string_lossy().into_owned(),
    })
}

#[cfg(windows)]
fn screenshots_dir() -> Result<PathBuf, String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Pictures, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath};

    unsafe {
        let raw = SHGetKnownFolderPath(
            &FOLDERID_Pictures,
            KNOWN_FOLDER_FLAG(0),
            Some(HANDLE::default()),
        )
        .map_err(|e| format!("dossier Images : {e}"))?;
        let dir = raw
            .to_string()
            .map_err(|e| format!("chemin Images : {e}"))?;
        CoTaskMemFree(Some(raw.0 as *const core::ffi::c_void));
        if dir.trim().is_empty() {
            return Err("dossier Images introuvable".into());
        }
        Ok(PathBuf::from(dir).join("Screenshots"))
    }
}

#[cfg(not(windows))]
fn screenshots_dir() -> Result<PathBuf, String> {
    Ok(std::env::temp_dir().join("pc-dashboard-screenshots"))
}

#[cfg(windows)]
fn timestamp() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;

    let stamp = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
        stamp.wYear, stamp.wMonth, stamp.wDay, stamp.wHour, stamp.wMinute, stamp.wSecond
    )
}

#[cfg(not(windows))]
fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_screenshots_dir() {
        let dir = screenshots_dir().expect("screenshots dir");
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn timestamp_is_filename_safe() {
        let stamp = timestamp();
        assert!(!stamp.is_empty());
        assert!(
            stamp
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn captures_and_saves_png() {
        let info = capture_to_file(true).expect("capture");
        assert!(info.width > 0 && info.height > 0);
        assert!(info.path.ends_with(".png"));
        assert!(PathBuf::from(&info.path).is_file());
        std::fs::remove_file(&info.path).expect("cleanup");
    }
}
