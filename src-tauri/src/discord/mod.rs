//! Mirrors the Windows microphone state on Discord.
//!
//! Discord ignores synthetic keystrokes, and `SET_VOICE_SETTINGS_2` over the
//! local RPC socket is rejected without an OAuth token, so the only reliable
//! path is the microphone toggle exposed by UI Automation. That toggle lives in
//! the user panel, at the bottom left of Discord's main window.
//!
//! Discord keeps that accessibility tree available while the window is
//! minimized or hidden, as long as the client has rendered it once. The window
//! is never shown: a client that never rendered cannot be reached this way, and
//! the interface then reports that Discord did not follow.

use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};

use sysinfo::{ProcessesToUpdate, System};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, TRUE};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTogglePattern,
    ToggleState_Off, ToggleState_On, TreeScope_Descendants, UIA_ButtonControlTypeId,
    UIA_TogglePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::core::BOOL;

/// Class fragment Discord puts on the microphone and deafen toggles. It does not
/// depend on the interface language.
const AUDIO_BUTTON_CLASS: &str = "audioButtonWithMenu";

/// The user panel sits in the bottom-left corner of the window. These bounds
/// are only used as a fallback when Discord renames its CSS classes.
const PANEL_MAX_WIDTH: i32 = 400;
const PANEL_MAX_HEIGHT: i32 = 120;

/// Give Discord a moment to expose the microphone button when its
/// accessibility tree is still being built.
const RENDER_TIMEOUT: Duration = Duration::from_millis(1500);
const RENDER_POLL: Duration = Duration::from_millis(100);

/// Discord applies the toggle quickly; the wait only covers the throttled
/// renderer of a hidden window.
const TOGGLE_TIMEOUT: Duration = Duration::from_millis(2000);
const TOGGLE_POLL: Duration = Duration::from_millis(200);

/// A window can expose several thousand elements; the user panel is always near
/// the beginning of the tree, so this cap keeps the search cheap.
const MAX_ELEMENTS: i32 = 3000;

/// Serializes toggles so two fast clicks cannot race on the same button.
static TOGGLE_LOCK: Mutex<()> = Mutex::new(());

pub enum SyncError {
    /// Discord is not running, so there is nothing to mirror.
    NotRunning,
    Failed(String),
}

/// Failure of one accessibility pass, before it is reported to the interface.
enum ApplyError {
    /// The microphone button is absent because Discord never rendered its tree.
    NotFound,
    Failed(String),
}

impl From<ApplyError> for SyncError {
    fn from(error: ApplyError) -> Self {
        match error {
            ApplyError::NotFound => {
                SyncError::Failed("Impossible de trouver le bouton micro de Discord.".into())
            }
            ApplyError::Failed(message) => SyncError::Failed(message),
        }
    }
}

struct ComGuard(bool);

impl ComGuard {
    fn new() -> Self {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE) };
        ComGuard(result.is_ok())
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

/// Whether a Discord window can be found, without touching its state.
pub fn is_available() -> bool {
    find_discord_window().is_some()
}

/// Puts Discord's microphone in the requested state.
pub fn set_muted(muted: bool) -> Result<(), SyncError> {
    let _guard = TOGGLE_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let _com = ComGuard::new();

    let window = find_discord_window().ok_or(SyncError::NotRunning)?;

    // The window is left untouched: showing a hidden Discord would make it
    // flash on every click, and a client that never rendered its accessibility
    // tree stays unreachable even when shown.
    apply_mute(window, muted).map_err(SyncError::from)
}

fn apply_mute(window: HWND, muted: bool) -> Result<(), ApplyError> {
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) }
            .map_err(|error| ApplyError::Failed(format!("UI Automation indisponible : {error}")))?;
    let root = unsafe { automation.ElementFromHandle(window) }
        .map_err(|_| ApplyError::Failed("Impossible de lire la fenêtre Discord.".into()))?;

    let mic = wait_for_mic(&automation, &root).ok_or(ApplyError::NotFound)?;
    let pattern =
        unsafe { mic.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId) }
            .map_err(|_| {
                ApplyError::Failed("Le bouton micro de Discord ne peut pas être actionné.".into())
            })?;

    let desired = if muted {
        ToggleState_On
    } else {
        ToggleState_Off
    };
    let current = unsafe { pattern.CurrentToggleState() }
        .map_err(|_| ApplyError::Failed("Impossible de lire l'état du micro Discord.".into()))?;
    if current == desired {
        return Ok(());
    }

    unsafe { pattern.Toggle() }
        .map_err(|_| ApplyError::Failed("Discord a refusé de changer l'état du micro.".into()))?;

    // Discord rebuilds the button when its state changes, so the original
    // element can go stale; look the toggle up again on every poll.
    let deadline = Instant::now() + TOGGLE_TIMEOUT;
    loop {
        if let Some(mic) = find_mic(&automation, &root) {
            if let Ok(pattern) = unsafe {
                mic.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
            } {
                if unsafe { pattern.CurrentToggleState() }.ok() == Some(desired) {
                    return Ok(());
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(ApplyError::Failed(
                "Discord n'a pas pris en compte le changement de micro.".into(),
            ));
        }
        sleep(TOGGLE_POLL);
    }
}

fn wait_for_mic(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
) -> Option<IUIAutomationElement> {
    let deadline = Instant::now() + RENDER_TIMEOUT;
    loop {
        if let Some(mic) = find_mic(automation, root) {
            return Some(mic);
        }
        if Instant::now() >= deadline {
            return None;
        }
        sleep(RENDER_POLL);
    }
}

fn find_mic(
    automation: &IUIAutomation,
    root: &IUIAutomationElement,
) -> Option<IUIAutomationElement> {
    let condition = unsafe { automation.CreateTrueCondition() }.ok()?;
    let elements = unsafe { root.FindAll(TreeScope_Descendants, &condition) }.ok()?;
    let count = unsafe { elements.Length() }.ok()?;
    let window = unsafe { root.CurrentBoundingRectangle() }.ok()?;

    let mut marked = Vec::new();
    let mut fallback = Vec::new();
    for index in 0..count.min(MAX_ELEMENTS) {
        let Ok(element) = (unsafe { elements.GetElement(index) }) else {
            continue;
        };
        if unsafe { element.CurrentControlType() }.ok() != Some(UIA_ButtonControlTypeId) {
            continue;
        }
        // Only toggle buttons can be the microphone; this also filters out the
        // chevrons that open the input and output device menus.
        if unsafe { element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId) }
            .is_err()
        {
            continue;
        }
        let Ok(rect) = (unsafe { element.CurrentBoundingRectangle() }) else {
            continue;
        };

        let class = unsafe { element.CurrentClassName() }
            .map(|class| class.to_string())
            .unwrap_or_default();
        if class.contains(AUDIO_BUTTON_CLASS) {
            marked.push((element, rect));
        } else if in_user_panel(rect, window) {
            fallback.push((element, rect));
        }
    }

    // The microphone is the left-most audio button of the user panel; the
    // deafen toggle sits to its right.
    leftmost(marked).or_else(|| leftmost(fallback))
}

fn leftmost(mut candidates: Vec<(IUIAutomationElement, RECT)>) -> Option<IUIAutomationElement> {
    candidates.sort_by_key(|(_, rect)| rect.left);
    candidates.into_iter().next().map(|(element, _)| element)
}

fn in_user_panel(rect: RECT, window: RECT) -> bool {
    rect.right > rect.left
        && rect.bottom > rect.top
        && rect.bottom > window.bottom - PANEL_MAX_HEIGHT
        && rect.left < window.left + PANEL_MAX_WIDTH
}

fn find_discord_window() -> Option<HWND> {
    let pids = discord_pids();
    if pids.is_empty() {
        return None;
    }

    let mut search = WindowSearch {
        pids: &pids,
        windows: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(collect_windows),
            LPARAM(&mut search as *mut WindowSearch as isize),
        );
    }

    let candidates: Vec<(HWND, String)> = search
        .windows
        .into_iter()
        .map(|window| (window, window_title(window)))
        .collect();

    candidates
        .iter()
        .filter(|(_, title)| title.to_lowercase().contains("discord"))
        .map(|(window, _)| *window)
        .max_by_key(|window| window_area(*window))
        .or_else(|| {
            candidates
                .iter()
                .map(|(window, _)| *window)
                .max_by_key(|window| window_area(*window))
        })
}

struct WindowSearch<'a> {
    pids: &'a [u32],
    windows: Vec<HWND>,
}

unsafe extern "system" fn collect_windows(window: HWND, lparam: LPARAM) -> BOOL {
    let search = unsafe { &mut *(lparam.0 as *mut WindowSearch) };
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    if search.pids.contains(&pid) {
        search.windows.push(window);
    }
    TRUE
}

fn discord_pids() -> Vec<u32> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .processes()
        .iter()
        .filter(|(_, process)| is_discord_process(&process.name().to_string_lossy()))
        .map(|(pid, _)| pid.as_u32())
        .collect()
}

fn is_discord_process(name: &str) -> bool {
    let name = name.trim();
    let stem = name
        .strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".EXE"))
        .unwrap_or(name);
    stem.to_ascii_lowercase().starts_with("discord")
}

fn window_title(window: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(window) };
    if length <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let written = unsafe { GetWindowTextW(window, &mut buffer) };
    String::from_utf16_lossy(&buffer[..written.max(0) as usize])
}

fn window_area(window: HWND) -> i64 {
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(window, &mut rect) }.is_err() {
        return 0;
    }
    let width = (rect.right - rect.left).max(0) as i64;
    let height = (rect.bottom - rect.top).max(0) as i64;
    width * height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_discord_processes() {
        assert!(is_discord_process("Discord.exe"));
        assert!(is_discord_process("discord"));
        assert!(is_discord_process("DiscordPTB.exe"));
        assert!(!is_discord_process("chrome.exe"));
        assert!(!is_discord_process(""));
    }

    #[test]
    fn targets_buttons_in_the_user_panel() {
        let window = RECT {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 800,
        };
        let mic = RECT {
            left: 30,
            top: 730,
            right: 62,
            bottom: 762,
        };
        let elsewhere = RECT {
            left: 500,
            top: 400,
            right: 532,
            bottom: 432,
        };
        let header = RECT {
            left: 30,
            top: 0,
            right: 62,
            bottom: 32,
        };
        assert!(in_user_panel(mic, window));
        assert!(!in_user_panel(elsewhere, window));
        assert!(!in_user_panel(header, window));
    }

    #[test]
    #[ignore = "requires a running Discord client"]
    fn finds_the_discord_window() {
        assert!(find_discord_window().is_some());
    }
}
