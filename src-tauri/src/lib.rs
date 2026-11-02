mod actions;
mod capture;
#[cfg(windows)]
mod discord;
mod events;
mod hardware;
mod state;

#[cfg(not(windows))]
mod discord {
    pub enum SyncError {
        NotRunning,
        Failed(String),
    }

    pub fn is_available() -> bool {
        false
    }

    pub fn set_muted(_muted: bool) -> Result<(), SyncError> {
        Err(SyncError::NotRunning)
    }
}

use std::time::Duration;

use tauri::{Emitter, Manager};

use capture::vitals::VitalsSampler;
use events::{MicInfo, VitalsPayload};
use state::VitalsState;

const POLL_INTERVAL_MS: u64 = 1000;

#[tauri::command]
fn get_vitals(state: tauri::State<'_, VitalsState>) -> Option<VitalsPayload> {
    state.get()
}

#[tauri::command]
fn get_hardware() -> hardware::HardwarePayload {
    hardware::collect()
}

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;

    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;

    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())?;
    } else {
        autolaunch.disable().map_err(|e| e.to_string())?;
    }
    autolaunch.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_mic_muted() -> Option<bool> {
    actions::audio::is_muted()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MicToggle {
    muted: bool,
    /// Whether Discord followed the change, or was not running. A failure never
    /// prevents the Windows microphone from being muted.
    discord_synced: bool,
}

#[tauri::command]
fn set_mic_muted(muted: bool) -> Result<bool, String> {
    actions::audio::set_muted(muted)
}

#[tauri::command]
fn toggle_mic() -> Result<MicToggle, String> {
    let current = actions::audio::is_muted().unwrap_or(false);
    let muted = actions::audio::set_muted(!current)?;

    // Discord keeps its own mute state: mirror the click on its microphone
    // toggle. Losing Discord must not undo the Windows mute, so the error is
    // only reported to the interface.
    let discord_synced = match discord::set_muted(muted) {
        Ok(()) | Err(discord::SyncError::NotRunning) => true,
        Err(discord::SyncError::Failed(error)) => {
            eprintln!("Discord mute sync failed: {error}");
            false
        }
    };

    Ok(MicToggle {
        muted,
        discord_synced,
    })
}

/// Whether Discord can be reached, without changing any setting.
#[tauri::command]
fn get_discord_status() -> bool {
    discord::is_available()
}

#[tauri::command]
fn get_keyboard_locked() -> bool {
    actions::keyboard::is_locked()
}

#[tauri::command]
fn set_keyboard_locked(
    app: tauri::AppHandle,
    locked: bool,
    hold_seconds: u64,
) -> Result<bool, String> {
    let hold = keyboard_hold(hold_seconds);
    let handle = app.clone();
    actions::keyboard::set_locked(locked, hold, move |unlocked| {
        // The lock lifts itself after the hold delay; tell the interface so
        // the tile stops showing a keyboard that works again.
        let _ = handle.emit("keyboard-lock", unlocked);
    })
}

#[tauri::command]
fn toggle_media() -> Result<bool, String> {
    actions::media::toggle_play_pause()
}

#[tauri::command]
fn media_next() -> Result<(), String> {
    actions::media::skip_next()
}

#[tauri::command]
fn media_previous() -> Result<(), String> {
    actions::media::skip_previous()
}

#[tauri::command]
fn open_media_source() -> Result<(), String> {
    actions::media::open_source()
}

#[tauri::command]
fn send_hotkey(shortcut: String) -> Result<(), String> {
    actions::hotkey::send(&shortcut)
}

#[tauri::command]
fn take_screenshot(
    app: tauri::AppHandle,
    all_screens: bool,
) -> Result<actions::screenshot::ScreenshotInfo, String> {
    use tauri_plugin_notification::NotificationExt;

    let info = actions::screenshot::capture_to_file(all_screens)?;
    let _ = app
        .notification()
        .builder()
        .title("Capture d'écran enregistrée")
        .body("La capture a été placée dans Images\\Screenshots.")
        .show();
    Ok(info)
}

/// Automatic release delay for the keyboard lock. `0` means no automatic
/// release: the keyboard stays locked until the button is used again.
fn keyboard_hold(seconds: u64) -> Option<Duration> {
    (seconds > 0).then(|| Duration::from_secs(seconds))
}

/// Whether the startup entry must be (re)written for `exe`.
///
/// - no marker: first launch, where autostart is on by default;
/// - marker for this executable: nothing to do;
/// - marker for another executable: repair the path, but only when autostart
///   is still on, because a missing entry can be an explicit opt-out.
fn needs_autostart_update(registered: Option<&str>, exe: &str, enabled: bool) -> bool {
    match registered {
        Some(path) => path != exe && enabled,
        None => true,
    }
}

/// Keeps the startup entry pointing at this executable.
///
/// Autostart is on by default, so the first release launch creates the entry.
/// The marker then records which executable was registered: when it differs
/// (the app was moved, or a debug build was registered before), the entry is
/// rewritten. An entry the user disabled is left alone, so opting out from the
/// settings page sticks across restarts.
fn enable_autostart_once(app: &tauri::App) {
    if cfg!(debug_assertions) {
        return;
    }

    use tauri_plugin_autostart::ManagerExt;

    let Ok(dir) = app.path().app_config_dir() else {
        return;
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let exe = exe.to_string_lossy().into_owned();

    let marker = dir.join("autostart-initialized");
    let registered = std::fs::read_to_string(&marker).ok();

    let autolaunch = app.autolaunch();
    if !needs_autostart_update(
        registered.as_deref(),
        &exe,
        autolaunch.is_enabled().unwrap_or(false),
    ) {
        return;
    }

    if autolaunch.enable().is_err() {
        return;
    }

    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(marker, exe);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(VitalsState::default())
        .invoke_handler(tauri::generate_handler![
            get_vitals,
            get_hardware,
            get_autostart,
            set_autostart,
            get_mic_muted,
            set_mic_muted,
            toggle_mic,
            get_discord_status,
            get_keyboard_locked,
            set_keyboard_locked,
            toggle_media,
            media_next,
            media_previous,
            open_media_source,
            send_hotkey,
            take_screenshot
        ])
        .setup(|app| {
            enable_autostart_once(app);

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut sampler = VitalsSampler::new();
                loop {
                    let mut payload = sampler.sample(POLL_INTERVAL_MS);
                    payload.mic = MicInfo {
                        muted: actions::audio::is_muted().unwrap_or(false),
                    };
                    if let Some(state) = handle.try_state::<VitalsState>() {
                        state.set(payload.clone());
                    }
                    let _ = handle.emit("vitals", payload);
                    std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Apps\PC Dashboard\pc-dashboard.exe";
    const OTHER: &str = r"C:\Build\pc-dashboard.exe";

    #[test]
    fn first_launch_registers_autostart() {
        assert!(needs_autostart_update(None, EXE, false));
    }

    #[test]
    fn matching_marker_needs_no_work() {
        assert!(!needs_autostart_update(Some(EXE), EXE, true));
        assert!(!needs_autostart_update(Some(EXE), EXE, false));
    }

    #[test]
    fn another_executable_is_repaired_while_autostart_is_on() {
        assert!(needs_autostart_update(Some(OTHER), EXE, true));
    }

    #[test]
    fn opt_out_survives_a_moved_executable() {
        assert!(!needs_autostart_update(Some(OTHER), EXE, false));
    }

    #[test]
    fn legacy_empty_marker_still_respects_opt_out() {
        // Releases before the path was stored wrote an empty marker; it must
        // not re-enable autostart after the user turned it off.
        assert!(!needs_autostart_update(Some(""), EXE, false));
        assert!(needs_autostart_update(Some(""), EXE, true));
    }

    #[test]
    fn zero_seconds_means_no_timer() {
        assert_eq!(keyboard_hold(0), None);
    }

    #[test]
    fn positive_seconds_become_the_hold_delay() {
        assert_eq!(keyboard_hold(60), Some(Duration::from_secs(60)));
        assert_eq!(keyboard_hold(900), Some(Duration::from_secs(900)));
    }
}
