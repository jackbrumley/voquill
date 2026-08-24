use crate::app::state::SessionState;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

fn char_to_vks(ch: char) -> (Vec<VIRTUAL_KEY>, bool) {
    match ch {
        'a'..='z' => (vec![VIRTUAL_KEY(ch.to_ascii_uppercase() as u16)], false),
        'A'..='Z' => (vec![VIRTUAL_KEY(ch as u16)], true),
        '0'..='9' => (vec![VIRTUAL_KEY(ch as u16)], false),
        ' ' => (vec![VK_SPACE], false),
        '.' => (vec![VK_OEM_PERIOD], false),
        ',' => (vec![VK_OEM_COMMA], false),
        ';' => (vec![VK_OEM_1], false),
        '/' => (vec![VK_OEM_2], false),
        '[' => (vec![VK_OEM_4], false),
        ']' => (vec![VK_OEM_6], false),
        '\\' => (vec![VK_OEM_5], false),
        '-' => (vec![VK_OEM_MINUS], false),
        '=' => (vec![VK_OEM_PLUS], false),
        '!' => (vec![VIRTUAL_KEY('1' as u16)], true),
        '@' => (vec![VIRTUAL_KEY('2' as u16)], true),
        '#' => (vec![VIRTUAL_KEY('3' as u16)], true),
        '$' => (vec![VIRTUAL_KEY('4' as u16)], true),
        '%' => (vec![VIRTUAL_KEY('5' as u16)], true),
        '^' => (vec![VIRTUAL_KEY('6' as u16)], true),
        '&' => (vec![VIRTUAL_KEY('7' as u16)], true),
        '*' => (vec![VIRTUAL_KEY('8' as u16)], true),
        '(' => (vec![VIRTUAL_KEY('9' as u16)], true),
        ')' => (vec![VIRTUAL_KEY('0' as u16)], true),
        '_' => (vec![VK_OEM_MINUS], true),
        '+' => (vec![VK_OEM_PLUS], true),
        '{' => (vec![VK_OEM_4], true),
        '}' => (vec![VK_OEM_6], true),
        '|' => (vec![VK_OEM_5], true),
        ':' => (vec![VK_OEM_1], true),
        '"' => (vec![VK_OEM_7], true),
        '<' => (vec![VK_OEM_COMMA], true),
        '>' => (vec![VK_OEM_PERIOD], true),
        '?' => (vec![VK_OEM_2], true),
        '~' => (vec![VK_OEM_3], true),
        '`' => (vec![VK_OEM_3], false),
        '\'' => (vec![VK_OEM_7], false),
        '\n' => (vec![VK_RETURN], false),
        '\t' => (vec![VK_TAB], false),
        _ => (vec![VK_SPACE], false),
    }
}

pub fn type_text_hardware(
    text: &str,
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
    session_state: &Arc<Mutex<SessionState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    crate::log_info!(
        "[Hardware Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
        text,
        interval_ms,
        key_press_duration_ms
    );

    for ch in text.chars() {
        // Best-effort cancel: stop typing if the session left the Typing phase.
        if *session_state.lock().unwrap() != SessionState::Typing {
            crate::log_info!("[Hardware Engine] Typing aborted: session cancelled");
            break;
        }

        let (vk_codes, needs_shift) = char_to_vks(ch);
        unsafe {
            if needs_shift {
                emit_vk(VK_SHIFT, true);
            }
            for vk in &vk_codes {
                emit_vk(*vk, true);
            }
            thread::sleep(hold_duration);
            for vk in &vk_codes {
                emit_vk(*vk, false);
            }
            if needs_shift {
                emit_vk(VK_SHIFT, false);
            }
        }
        if interval_ms > 0 {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }

    crate::log_info!("Hardware typing complete");
    Ok(())
}

pub fn send_paste_shortcut(
    shortcut: crate::config::PasteShortcut,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    crate::log_info!("[Windows] Sending paste shortcut: {:?}", shortcut);
    unsafe {
        // Safety clear: release modifiers first to clear any latent modifier state
        emit_vk(VK_CONTROL, false);
        emit_vk(VK_SHIFT, false);
        emit_vk(VK_MENU, false);
        std::thread::sleep(std::time::Duration::from_millis(10));

        match shortcut {
            crate::config::PasteShortcut::ShiftInsert => {
                emit_vk(VK_SHIFT, true);
                std::thread::sleep(std::time::Duration::from_millis(15));
                emit_vk(VK_INSERT, true);
                std::thread::sleep(std::time::Duration::from_millis(25));
                emit_vk(VK_INSERT, false);
                std::thread::sleep(std::time::Duration::from_millis(15));
                emit_vk(VK_SHIFT, false);
            }
            crate::config::PasteShortcut::CtrlV => {
                emit_vk(VK_CONTROL, true);
                std::thread::sleep(std::time::Duration::from_millis(15));
                emit_vk(VIRTUAL_KEY('V' as u16), true);
                std::thread::sleep(std::time::Duration::from_millis(25));
                emit_vk(VIRTUAL_KEY('V' as u16), false);
                std::thread::sleep(std::time::Duration::from_millis(15));
                emit_vk(VK_CONTROL, false);
            }
            crate::config::PasteShortcut::CtrlShiftV => {
                emit_vk(VK_CONTROL, true);
                emit_vk(VK_SHIFT, true);
                std::thread::sleep(std::time::Duration::from_millis(15));
                emit_vk(VIRTUAL_KEY('V' as u16), true);
                std::thread::sleep(std::time::Duration::from_millis(25));
                emit_vk(VIRTUAL_KEY('V' as u16), false);
                std::thread::sleep(std::time::Duration::from_millis(15));
                emit_vk(VK_SHIFT, false);
                emit_vk(VK_CONTROL, false);
            }
        }
    }
    crate::log_info!("[Windows] Paste shortcut complete");
    Ok(())
}

pub fn parse_windows_vk(token: &str) -> Result<VIRTUAL_KEY, String> {
    let lower = token.trim().to_lowercase();
    let stripped = lower.strip_prefix("key").unwrap_or(&lower);
    let stripped_digit = stripped.strip_prefix("digit").unwrap_or(stripped);
    let stripped_num = stripped_digit.strip_prefix("num").unwrap_or(stripped_digit);

    if stripped_num.len() == 1 {
        let ch = stripped_num.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Ok(VIRTUAL_KEY(ch.to_ascii_uppercase() as u16));
        }
        if ch.is_ascii_digit() {
            return Ok(VIRTUAL_KEY(ch as u16));
        }
        match ch {
            ' ' => return Ok(VK_SPACE),
            '.' => return Ok(VK_OEM_PERIOD),
            ',' => return Ok(VK_OEM_COMMA),
            ';' => return Ok(VK_OEM_1),
            '/' => return Ok(VK_OEM_2),
            '[' => return Ok(VK_OEM_4),
            ']' => return Ok(VK_OEM_6),
            '\\' => return Ok(VK_OEM_5),
            '-' => return Ok(VK_OEM_MINUS),
            '=' => return Ok(VK_OEM_PLUS),
            '`' => return Ok(VK_OEM_3),
            '\'' => return Ok(VK_OEM_7),
            _ => {}
        }
    }

    match lower.as_str() {
        "f1" => Ok(VK_F1),
        "f2" => Ok(VK_F2),
        "f3" => Ok(VK_F3),
        "f4" => Ok(VK_F4),
        "f5" => Ok(VK_F5),
        "f6" => Ok(VK_F6),
        "f7" => Ok(VK_F7),
        "f8" => Ok(VK_F8),
        "f9" => Ok(VK_F9),
        "f10" => Ok(VK_F10),
        "f11" => Ok(VK_F11),
        "f12" => Ok(VK_F12),
        "f13" => Ok(VK_F13),
        "f14" => Ok(VK_F14),
        "f15" => Ok(VK_F15),
        "f16" => Ok(VK_F16),
        "f17" => Ok(VK_F17),
        "f18" => Ok(VK_F18),
        "f19" => Ok(VK_F19),
        "f20" => Ok(VK_F20),
        "f21" => Ok(VK_F21),
        "f22" => Ok(VK_F22),
        "f23" => Ok(VK_F23),
        "f24" => Ok(VK_F24),
        "space" => Ok(VK_SPACE),
        "enter" | "return" => Ok(VK_RETURN),
        "tab" => Ok(VK_TAB),
        "esc" | "escape" => Ok(VK_ESCAPE),
        "backspace" => Ok(VK_BACK),
        "delete" | "del" => Ok(VK_DELETE),
        "insert" | "ins" => Ok(VK_INSERT),
        "home" => Ok(VK_HOME),
        "end" => Ok(VK_END),
        "pageup" | "pgup" => Ok(VK_PRIOR),
        "pagedown" | "pgdn" => Ok(VK_NEXT),
        "up" | "arrowup" => Ok(VK_UP),
        "down" | "arrowdown" => Ok(VK_DOWN),
        "left" | "arrowleft" => Ok(VK_LEFT),
        "right" | "arrowright" => Ok(VK_RIGHT),
        "ctrl" | "control" | "lctrl" | "rctrl" => Ok(VK_CONTROL),
        "shift" | "lshift" | "rshift" => Ok(VK_SHIFT),
        "alt" | "menu" | "lalt" | "ralt" => Ok(VK_MENU),
        "super" | "win" | "cmd" | "meta" => Ok(VK_LWIN),
        _ => Err(format!(
            "Unrecognized key token '{token}'. Expected a valid key (e.g. F1-F24, Ctrl, Alt, Shift, Super, A-Z, 0-9, Space, Enter, Tab, Escape, etc.)"
        )),
    }
}

pub fn send_key_combination(
    combination: &str,
    hold_duration_ms: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let hold_duration = Duration::from_millis(hold_duration_ms.max(20));
    let parts: Vec<&str> = combination
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("Empty key combination".into());
    }

    let mut modifier_vks = Vec::new();
    let mut main_vks = Vec::new();

    for part in parts {
        let vk = parse_windows_vk(part).map_err(|e| e)?;
        if vk == VK_CONTROL || vk == VK_SHIFT || vk == VK_MENU || vk == VK_LWIN {
            if !modifier_vks.contains(&vk) {
                modifier_vks.push(vk);
            }
        } else {
            main_vks.push(vk);
        }
    }

    crate::log_info!(
        "[Windows] Sending key combination: '{}' (modifiers: {}, keys: {}, hold: {}ms)",
        combination,
        modifier_vks.len(),
        main_vks.len(),
        hold_duration_ms
    );

    unsafe {
        // Safety: release modifiers first to clear latent state
        emit_vk(VK_CONTROL, false);
        emit_vk(VK_SHIFT, false);
        emit_vk(VK_MENU, false);
        emit_vk(VK_LWIN, false);
        thread::sleep(Duration::from_millis(10));

        for mod_vk in &modifier_vks {
            emit_vk(*mod_vk, true);
            thread::sleep(Duration::from_millis(10));
        }

        for vk in &main_vks {
            emit_vk(*vk, true);
        }

        thread::sleep(hold_duration);

        for vk in main_vks.iter().rev() {
            emit_vk(*vk, false);
        }

        thread::sleep(Duration::from_millis(10));

        for mod_vk in modifier_vks.iter().rev() {
            emit_vk(*mod_vk, false);
        }
    }

    crate::log_info!("[Windows] Key combination complete");
    Ok(())
}

pub fn send_key_down(key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let vk = parse_windows_vk(key)?;
    crate::log_info!("[Windows] Sending KeyDown: '{}' (VK: {:?})", key, vk);
    unsafe {
        emit_vk(vk, true);
    }
    Ok(())
}

pub fn send_key_up(key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let vk = parse_windows_vk(key)?;
    crate::log_info!("[Windows] Sending KeyUp: '{}' (VK: {:?})", key, vk);
    unsafe {
        emit_vk(vk, false);
    }
    Ok(())
}

fn is_extended_key(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_INSERT
            | VK_DELETE
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_UP
            | VK_DOWN
            | VK_LEFT
            | VK_RIGHT
            | VK_RCONTROL
            | VK_RMENU
            | VK_DIVIDE
            | VK_NUMLOCK
    )
}

unsafe fn emit_vk(vk: VIRTUAL_KEY, is_down: bool) {
    let mut dw_flags = if is_down {
        KEYBD_EVENT_FLAGS(0)
    } else {
        KEYEVENTF_KEYUP
    };
    if is_extended_key(vk) {
        dw_flags |= KEYEVENTF_EXTENDEDKEY;
    }

    let mut input = INPUT {
        r#type: INPUT_KEYBOARD,
        ..Default::default()
    };
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: dw_flags,
        time: 0,
        dwExtraInfo: 0,
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_extended_key() {
        assert!(is_extended_key(VK_INSERT));
        assert!(is_extended_key(VK_DELETE));
        assert!(is_extended_key(VK_HOME));
        assert!(is_extended_key(VK_END));
        assert!(is_extended_key(VK_PRIOR));
        assert!(is_extended_key(VK_NEXT));
        assert!(is_extended_key(VK_UP));
        assert!(is_extended_key(VK_DOWN));
        assert!(is_extended_key(VK_LEFT));
        assert!(is_extended_key(VK_RIGHT));

        assert!(!is_extended_key(VK_SHIFT));
        assert!(!is_extended_key(VK_CONTROL));
        assert!(!is_extended_key(VIRTUAL_KEY('V' as u16)));
        assert!(!is_extended_key(VK_SPACE));
    }
}
