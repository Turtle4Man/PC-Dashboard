use std::future::IntoFuture;

use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Win32::Foundation::{HWND, LPARAM, TRUE};
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_DISABLE_OLE1DDE, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationSelectionItemPattern,
    TreeScope_Descendants, UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::BOOL;
use windows_future::IAsyncOperation;

struct ComGuard(bool);

impl ComGuard {
    fn new() -> Self {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED | COINIT_DISABLE_OLE1DDE) };
        ComGuard(hr.0 == 0)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

fn block<T: windows::core::RuntimeType>(operation: IAsyncOperation<T>) -> windows::core::Result<T> {
    pollster::block_on(operation.into_future())
}

pub fn toggle_play_pause() -> Result<bool, String> {
    let _com = ComGuard::new();
    let session = current_session()?;
    let toggled = session
        .TryTogglePlayPauseAsync()
        .and_then(block)
        .map_err(|e| format!("bascule média: {e}"))?;
    if !toggled {
        return Err("la session média a refusé le changement".into());
    }
    session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .map_err(|e| format!("état média: {e}"))
}

pub fn skip_next() -> Result<(), String> {
    let _com = ComGuard::new();
    let session = current_session()?;
    session
        .TrySkipNextAsync()
        .and_then(block)
        .map(|_| ())
        .map_err(|e| format!("piste suivante : {e}"))
}

pub fn skip_previous() -> Result<(), String> {
    let _com = ComGuard::new();
    let session = current_session()?;
    session
        .TrySkipPreviousAsync()
        .and_then(block)
        .map(|_| ())
        .map_err(|e| format!("piste précédente : {e}"))
}

fn current_session() -> Result<GlobalSystemMediaTransportControlsSession, String> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .and_then(block)
        .map_err(|e| format!("session média: {e}"))?;
    manager
        .GetCurrentSession()
        .map_err(|_| "aucune lecture en cours".to_string())
}

pub fn open_source() -> Result<(), String> {
    let _com = ComGuard::new();
    let session = current_session()?;
    let aumid = session
        .SourceAppUserModelId()
        .map(|id| id.to_string())
        .unwrap_or_default();
    if aumid.trim().is_empty() {
        return Err("source du média inconnue".into());
    }
    let title = session
        .TryGetMediaPropertiesAsync()
        .and_then(block)
        .ok()
        .and_then(|props| props.Title().ok())
        .map(|title| title.to_string())
        .unwrap_or_default();
    if focus_media_tab(&aumid, &title) {
        return Ok(());
    }
    if focus_app_window(&aumid) {
        return Ok(());
    }
    // Repli : activation (peut ouvrir une nouvelle fenêtre).
    std::process::Command::new("explorer.exe")
        .arg(format!("shell:AppsFolder\\{aumid}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("ouverture de la source : {e}"))
}

fn tab_matches(tab_name: &str, media_title: &str) -> bool {
    let tab = tab_name.to_lowercase();
    let title = media_title.trim().to_lowercase();
    !tab.trim().is_empty()
        && !title.is_empty()
        && (tab.contains(&title) || title.contains(tab.trim()))
}

// Marqueurs "onglet audible" (dépendent de la langue et du navigateur).
const AUDIO_MARKERS: [&str; 2] = ["lecture audio", "playing audio"];

fn tab_is_audible(tab_name: &str) -> bool {
    let tab = tab_name.to_lowercase();
    AUDIO_MARKERS.iter().any(|marker| tab.contains(marker))
}

const MAX_UIA_ELEMENTS: i32 = 3000;

fn focus_media_tab(aumid: &str, title: &str) -> bool {
    let automation: IUIAutomation =
        match unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) } {
            Ok(automation) => automation,
            Err(_) => return false,
        };
    for hwnd in visible_top_level_windows() {
        if window_aumid(hwnd).as_deref() != Some(aumid) {
            continue;
        }
        if activate_matching_tab(&automation, hwnd, title) {
            return restore_and_focus(hwnd);
        }
    }
    false
}

fn tab_items(automation: &IUIAutomation, hwnd: HWND) -> Vec<(IUIAutomationElement, String)> {
    let mut out = Vec::new();
    unsafe {
        let Ok(root) = automation.ElementFromHandle(hwnd) else {
            return out;
        };
        let Ok(condition) = automation.CreateTrueCondition() else {
            return out;
        };
        let Ok(all) = root.FindAll(TreeScope_Descendants, &condition) else {
            return out;
        };
        let Ok(len) = all.Length() else {
            return out;
        };
        for index in 0..len.min(MAX_UIA_ELEMENTS) {
            let Ok(element) = all.GetElement(index) else {
                continue;
            };
            if element.CurrentControlType().ok() != Some(UIA_TabItemControlTypeId) {
                continue;
            }
            let name = element
                .CurrentName()
                .map(|name| name.to_string())
                .unwrap_or_default();
            out.push((element, name));
        }
    }
    out
}

fn activate_matching_tab(automation: &IUIAutomation, hwnd: HWND, title: &str) -> bool {
    let tabs = tab_items(automation, hwnd);
    if let Some((element, _)) = tabs.iter().find(|(_, name)| tab_matches(name, title)) {
        return select_tab(element);
    }
    tabs.into_iter()
        .find(|(_, name)| tab_is_audible(name))
        .map(|(element, _)| select_tab(&element))
        .unwrap_or(false)
}

fn select_tab(element: &IUIAutomationElement) -> bool {
    unsafe {
        let Ok(pattern) = element
            .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId)
        else {
            return false;
        };
        if pattern.Select().is_err() {
            return false;
        }
        pattern
            .CurrentIsSelected()
            .map(|selected| selected.as_bool())
            .unwrap_or(false)
    }
}

fn restore_and_focus(hwnd: HWND) -> bool {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd).as_bool()
    }
}

fn focus_app_window(aumid: &str) -> bool {
    for hwnd in visible_top_level_windows() {
        if window_aumid(hwnd).as_deref() != Some(aumid) {
            continue;
        }
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            if SetForegroundWindow(hwnd).as_bool() {
                return true;
            }
        }
    }
    false
}

fn visible_top_level_windows() -> Vec<HWND> {
    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            out.push(hwnd);
        }
        TRUE
    }

    let mut out = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut out as *mut Vec<HWND> as isize));
    }
    out
}

fn window_aumid(hwnd: HWND) -> Option<String> {
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd).ok()?;
        let value = store.GetValue(&PKEY_AppUserModel_ID as *const _).ok()?;
        let text = PropVariantToStringAlloc(&value).ok()?;
        let name = text.to_string().unwrap_or_default();
        CoTaskMemFree(Some(text.0 as *const core::ffi::c_void));
        (!name.is_empty()).then_some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_visible_windows() {
        let _com = ComGuard::new();
        assert!(!visible_top_level_windows().is_empty());
    }

    #[test]
    fn reads_window_aumids_without_crashing() {
        let _com = ComGuard::new();
        for hwnd in visible_top_level_windows().into_iter().take(20) {
            let _ = window_aumid(hwnd);
        }
    }

    #[test]
    fn matches_media_title_in_tab_name() {
        assert!(tab_matches(
            "Pokémon Reborn - Champion Battle Theme - YouTube - Lecture audio",
            "Pokémon Reborn - Champion Battle Theme"
        ));
        assert!(tab_matches("some video - youtube", "SOME VIDEO"));
        assert!(!tab_matches("Gmail - Inbox", "Pokémon Reborn"));
        assert!(!tab_matches("", "Something"));
        assert!(!tab_matches("Something", ""));
        assert!(!tab_matches("Something", "   "));
    }

    #[test]
    fn detects_audible_marker() {
        assert!(tab_is_audible("Video - YouTube - Lecture audio"));
        assert!(tab_is_audible("Video - Playing audio"));
        assert!(!tab_is_audible("Gmail - Inbox"));
    }

    #[test]
    fn finds_browser_tabs_without_touching_them() {
        let _com = ComGuard::new();
        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).unwrap() };
        let tabs: Vec<String> = visible_top_level_windows()
            .into_iter()
            .flat_map(|hwnd| tab_items(&automation, hwnd))
            .map(|(_, name)| name)
            .collect();
        assert!(!tabs.is_empty());
    }
}
