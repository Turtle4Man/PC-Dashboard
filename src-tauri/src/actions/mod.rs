#[cfg(windows)]
pub mod audio;

#[cfg(windows)]
pub mod media;

#[cfg(windows)]
pub mod hotkey;

#[cfg(windows)]
pub mod keyboard;

pub mod screenshot;

#[cfg(not(windows))]
pub mod audio {
    pub fn is_muted() -> Option<bool> {
        None
    }

    pub fn set_muted(_muted: bool) -> Result<bool, String> {
        Err("Core Audio is only available on Windows".into())
    }
}

#[cfg(not(windows))]
pub mod hotkey {
    pub fn send(_shortcut: &str) -> Result<(), String> {
        Err("Les raccourcis clavier ne sont disponibles que sous Windows".into())
    }
}

#[cfg(not(windows))]
pub mod keyboard {
    use std::time::Duration;

    pub fn is_locked() -> bool {
        false
    }

    pub fn set_locked(
        _locked: bool,
        _hold: Option<Duration>,
        _on_unlock: impl Fn(bool) + Send + 'static,
    ) -> Result<bool, String> {
        Err("Le verrouillage du clavier n'est disponible que sous Windows".into())
    }
}

#[cfg(not(windows))]
pub mod media {
    pub fn toggle_play_pause() -> Result<bool, String> {
        Err("Media control is only available on Windows".into())
    }

    pub fn skip_next() -> Result<(), String> {
        Err("Media control is only available on Windows".into())
    }

    pub fn skip_previous() -> Result<(), String> {
        Err("Media control is only available on Windows".into())
    }

    pub fn open_source() -> Result<(), String> {
        Err("Media control is only available on Windows".into())
    }
}
