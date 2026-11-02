use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_0, VK_A, VK_APPS, VK_BACK, VK_CAPITAL, VK_CONTROL,
    VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_HOME, VK_INSERT, VK_LEFT, VK_LWIN,
    VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU, VK_NEXT,
    VK_NUMLOCK, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RWIN, VK_SCROLL,
    VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};

const FUNCTION_KEY_COUNT: u16 = 24;

pub fn send(shortcut: &str) -> Result<(), String> {
    send_keys(&parse(shortcut)?)
}

pub fn send_keys(keys: &[VIRTUAL_KEY]) -> Result<(), String> {
    if keys.is_empty() {
        return Err("Le raccourci est vide.".into());
    }

    let mut inputs = Vec::with_capacity(keys.len() * 2);
    for key in keys {
        inputs.push(key_event(*key, false));
    }
    for key in keys.iter().rev() {
        inputs.push(key_event(*key, true));
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err("Windows a refusé d'envoyer le raccourci.".into());
    }
    Ok(())
}

fn key_event(key: VIRTUAL_KEY, released: bool) -> INPUT {
    let mut flags = if released {
        KEYEVENTF_KEYUP
    } else {
        KEYBD_EVENT_FLAGS(0)
    };
    if is_extended_key(key) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

// Extended keys share a virtual-key code with a numpad key, so Windows needs
// the extended flag to target the right one.
fn is_extended_key(key: VIRTUAL_KEY) -> bool {
    matches!(
        key,
        VK_INSERT
            | VK_DELETE
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_LEFT
            | VK_RIGHT
            | VK_UP
            | VK_DOWN
            | VK_NUMLOCK
            | VK_DIVIDE
            | VK_SNAPSHOT
            | VK_PAUSE
            | VK_LWIN
            | VK_RWIN
            | VK_APPS
            | VK_RCONTROL
            | VK_RMENU
            | VK_MEDIA_PLAY_PAUSE
            | VK_MEDIA_NEXT_TRACK
            | VK_MEDIA_PREV_TRACK
            | VK_MEDIA_STOP
            | VK_VOLUME_UP
            | VK_VOLUME_DOWN
            | VK_VOLUME_MUTE
    )
}

fn parse(shortcut: &str) -> Result<Vec<VIRTUAL_KEY>, String> {
    let mut modifiers = Vec::new();
    let mut final_key = None;

    for raw in shortcut.split('+') {
        let token = raw.trim().to_ascii_lowercase();
        if token.is_empty() {
            continue;
        }

        let modifier = match token.as_str() {
            "ctrl" | "control" => Some(VK_CONTROL),
            "alt" => Some(VK_MENU),
            "shift" => Some(VK_SHIFT),
            "win" | "meta" | "super" => Some(VK_LWIN),
            _ => None,
        };

        if let Some(modifier) = modifier {
            if !modifiers.contains(&modifier) {
                modifiers.push(modifier);
            }
            continue;
        }

        let key = virtual_key(&token).ok_or_else(|| format!("Touche inconnue : {raw}"))?;
        if final_key.replace(key).is_some() {
            return Err("Un raccourci ne peut contenir qu'une seule touche finale.".into());
        }
    }

    let key = final_key.ok_or("Le raccourci doit contenir une touche.")?;
    modifiers.push(key);
    Ok(modifiers)
}

fn virtual_key(token: &str) -> Option<VIRTUAL_KEY> {
    if token.len() == 1 {
        let character = token.chars().next()?;
        if character.is_ascii_alphabetic() {
            let offset = character.to_ascii_uppercase() as u16 - 'A' as u16;
            return Some(VIRTUAL_KEY(VK_A.0 + offset));
        }
        if character.is_ascii_digit() {
            let offset = character as u16 - '0' as u16;
            return Some(VIRTUAL_KEY(VK_0.0 + offset));
        }
    }

    if let Some(number) = token
        .strip_prefix('f')
        .and_then(|value| value.parse::<u16>().ok())
    {
        if (1..=FUNCTION_KEY_COUNT).contains(&number) {
            return Some(VIRTUAL_KEY(VK_F1.0 + number - 1));
        }
    }

    Some(match token {
        "space" => VK_SPACE,
        "enter" | "return" => VK_RETURN,
        "tab" => VK_TAB,
        "escape" | "esc" => VK_ESCAPE,
        "backspace" => VK_BACK,
        "delete" | "del" => VK_DELETE,
        "insert" | "ins" => VK_INSERT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" | "pgup" => VK_PRIOR,
        "pagedown" | "pgdn" => VK_NEXT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "printscreen" | "prtsc" => VK_SNAPSHOT,
        "pause" => VK_PAUSE,
        "capslock" => VK_CAPITAL,
        "numlock" => VK_NUMLOCK,
        "scrolllock" => VK_SCROLL,
        "playpause" | "mediaplaypause" => VK_MEDIA_PLAY_PAUSE,
        "nexttrack" | "medianext" => VK_MEDIA_NEXT_TRACK,
        "prevtrack" | "mediaprev" => VK_MEDIA_PREV_TRACK,
        "volumeup" => VK_VOLUME_UP,
        "volumedown" => VK_VOLUME_DOWN,
        "mute" | "volumemute" => VK_VOLUME_MUTE,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> VIRTUAL_KEY {
        virtual_key(name).expect("known key")
    }

    #[test]
    fn parses_modifiers_and_letter() {
        assert_eq!(
            parse("Ctrl+Shift+M").expect("parse"),
            vec![VK_CONTROL, VK_SHIFT, key("m")]
        );
    }

    #[test]
    fn parses_function_keys_and_digits() {
        assert_eq!(
            parse("alt+f12").expect("parse"),
            vec![VK_MENU, VIRTUAL_KEY(VK_F1.0 + 11)]
        );
        assert_eq!(
            parse("ctrl+1").expect("parse"),
            vec![VK_CONTROL, VIRTUAL_KEY(VK_0.0 + 1)]
        );
    }

    #[test]
    fn parses_named_keys() {
        assert_eq!(parse("Win+Space").expect("parse"), vec![VK_LWIN, VK_SPACE]);
        assert_eq!(
            parse("Ctrl+PageDown").expect("parse"),
            vec![VK_CONTROL, VK_NEXT]
        );
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(parse("Ctrl+Banana").is_err());
        assert!(parse("Ctrl+F25").is_err());
    }

    #[test]
    fn requires_a_final_key() {
        assert!(parse("").is_err());
        assert!(parse("Ctrl+Shift").is_err());
    }

    #[test]
    fn rejects_several_final_keys() {
        assert!(parse("Ctrl+A+B").is_err());
    }

    #[test]
    fn marks_extended_keys() {
        assert!(is_extended_key(VK_LEFT));
        assert!(is_extended_key(VK_DELETE));
        assert!(is_extended_key(VK_VOLUME_UP));
        assert!(is_extended_key(VK_LWIN));
        assert!(!is_extended_key(VK_A));
        assert!(!is_extended_key(VK_CONTROL));
    }
}
