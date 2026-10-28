use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::app::state::SessionState;
use crate::config::PasteShortcut;

type CGEventRef = *mut std::ffi::c_void;

const KCG_HID_EVENT_TAP: u32 = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *const std::ffi::c_void,
        key_code: u16,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventKeyboardSetUnicodeString(
        event: CGEventRef,
        string_length: u64,
        unicode_string: *const u16,
    );
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CFRelease(value: *const std::ffi::c_void);
}

fn require_accessibility() -> Result<(), String> {
    if super::permissions::accessibility_is_trusted() {
        Ok(())
    } else {
        Err("Voquill needs Accessibility permission to send keyboard input. Open Setup and grant it in System Settings.".to_string())
    }
}

fn post_key(key_code: u16, key_down: bool) -> Result<(), String> {
    // SAFETY: Quartz creates and owns the event; it is released after posting.
    unsafe {
        let event = CGEventCreateKeyboardEvent(ptr::null(), key_code, key_down);
        if event.is_null() {
            return Err("Failed to create macOS keyboard event".to_string());
        }
        CGEventPost(KCG_HID_EVENT_TAP, event);
        CFRelease(event);
    }
    Ok(())
}

fn post_unicode(character: char) -> Result<(), String> {
    let mut units = [0u16; 2];
    let encoded = character.encode_utf16(&mut units);
    // SAFETY: Quartz copies the supplied UTF-16 sequence into the event before it is posted.
    unsafe {
        let key_down = CGEventCreateKeyboardEvent(ptr::null(), 0, true);
        let key_up = CGEventCreateKeyboardEvent(ptr::null(), 0, false);
        if key_down.is_null() || key_up.is_null() {
            if !key_down.is_null() {
                CFRelease(key_down);
            }
            if !key_up.is_null() {
                CFRelease(key_up);
            }
            return Err("Failed to create macOS Unicode keyboard event".to_string());
        }
        CGEventKeyboardSetUnicodeString(key_down, encoded.len() as u64, encoded.as_ptr());
        CGEventKeyboardSetUnicodeString(key_up, encoded.len() as u64, encoded.as_ptr());
        CGEventPost(KCG_HID_EVENT_TAP, key_down);
        CGEventPost(KCG_HID_EVENT_TAP, key_up);
        CFRelease(key_down);
        CFRelease(key_up);
    }
    Ok(())
}

pub fn type_text_hardware(
    text: &str,
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
    session_state: &Arc<Mutex<SessionState>>,
) -> Result<(), String> {
    require_accessibility()?;
    let interval = Duration::from_secs_f64(typing_speed_interval.max(0.0));
    let hold = Duration::from_millis(key_press_duration_ms);

    for character in text.chars() {
        if *session_state.lock().unwrap() != SessionState::Typing {
            crate::log_info!("macOS typing aborted: session cancelled");
            break;
        }
        post_unicode(character)?;
        if !hold.is_zero() {
            thread::sleep(hold);
        }
        if !interval.is_zero() {
            thread::sleep(interval);
        }
    }
    Ok(())
}

pub fn send_paste_shortcut(shortcut: PasteShortcut) -> Result<(), String> {
    if shortcut != PasteShortcut::CommandV {
        return Err(
            "macOS supports Command+V for automatic paste. Select Command + V in Typing settings."
                .to_string(),
        );
    }
    send_key_combination("Command+V", 25)
}

pub fn send_key_combination(combination: &str, hold_duration_ms: u64) -> Result<(), String> {
    require_accessibility()?;
    let keys = combination
        .split('+')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(parse_key)
        .collect::<Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Err("Empty key combination".to_string());
    }

    let (modifiers, keys): (Vec<_>, Vec<_>) = keys.into_iter().partition(|key| key.modifier);
    for key in &modifiers {
        post_key(key.code, true)?;
    }
    for key in &keys {
        post_key(key.code, true)?;
    }
    thread::sleep(Duration::from_millis(hold_duration_ms.max(20)));
    for key in keys.iter().rev() {
        post_key(key.code, false)?;
    }
    for key in modifiers.iter().rev() {
        post_key(key.code, false)?;
    }
    Ok(())
}

pub fn send_key_down(key: &str) -> Result<(), String> {
    require_accessibility()?;
    post_key(parse_key(key)?.code, true)
}

pub fn send_key_up(key: &str) -> Result<(), String> {
    require_accessibility()?;
    post_key(parse_key(key)?.code, false)
}

pub(super) struct Key {
    pub(super) code: u16,
    pub(super) modifier: bool,
}

pub(super) fn parse_key(token: &str) -> Result<Key, String> {
    let key = token.trim().to_ascii_lowercase();
    let key = key
        .strip_prefix("key")
        .or_else(|| key.strip_prefix("digit"))
        .unwrap_or(&key);
    let (code, modifier) = match key {
        "command" | "cmd" | "super" | "win" | "meta" => (55, true),
        "control" | "ctrl" | "lctrl" | "rctrl" => (59, true),
        "option" | "alt" | "lalt" | "ralt" => (58, true),
        "shift" | "lshift" | "rshift" => (56, true),
        "a" => (0, false),
        "s" => (1, false),
        "d" => (2, false),
        "f" => (3, false),
        "h" => (4, false),
        "g" => (5, false),
        "z" => (6, false),
        "x" => (7, false),
        "c" => (8, false),
        "v" => (9, false),
        "b" => (11, false),
        "q" => (12, false),
        "w" => (13, false),
        "e" => (14, false),
        "r" => (15, false),
        "y" => (16, false),
        "t" => (17, false),
        "1" => (18, false),
        "2" => (19, false),
        "3" => (20, false),
        "4" => (21, false),
        "6" => (22, false),
        "5" => (23, false),
        "=" => (24, false),
        "9" => (25, false),
        "7" => (26, false),
        "-" => (27, false),
        "8" => (28, false),
        "0" => (29, false),
        "]" => (30, false),
        "o" => (31, false),
        "u" => (32, false),
        "[" => (33, false),
        "i" => (34, false),
        "p" => (35, false),
        "l" => (37, false),
        "j" => (38, false),
        "'" => (39, false),
        "k" => (40, false),
        ";" => (41, false),
        "\\" => (42, false),
        "," => (43, false),
        "/" => (44, false),
        "n" => (45, false),
        "m" => (46, false),
        "." => (47, false),
        "tab" => (48, false),
        "space" => (49, false),
        "`" => (50, false),
        "backspace" | "delete" => (51, false),
        "enter" | "return" => (36, false),
        "escape" | "esc" => (53, false),
        "f1" => (122, false),
        "f2" => (120, false),
        "f3" => (99, false),
        "f4" => (118, false),
        "f5" => (96, false),
        "f6" => (97, false),
        "f7" => (98, false),
        "f8" => (100, false),
        "f9" => (101, false),
        "f10" => (109, false),
        "f11" => (103, false),
        "f12" => (111, false),
        "left" | "arrowleft" => (123, false),
        "right" | "arrowright" => (124, false),
        "down" | "arrowdown" => (125, false),
        "up" | "arrowup" => (126, false),
        _ => return Err(format!("Unrecognized macOS key token '{token}'")),
    };
    Ok(Key { code, modifier })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_macos_command_shortcut() {
        let command = parse_key("Command").unwrap();
        let v = parse_key("V").unwrap();
        assert_eq!(command.code, 55);
        assert!(command.modifier);
        assert_eq!(v.code, 9);
        assert!(!v.modifier);
    }
}
