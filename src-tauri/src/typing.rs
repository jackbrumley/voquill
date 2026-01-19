#[cfg(target_os = "linux")]
use evdev::{KeyCode, KeyEvent, SynchronizationEvent, SynchronizationCode};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;

#[cfg(target_os = "linux")]
fn char_to_keys(ch: char) -> (Vec<KeyCode>, bool) {
    match ch {
        'a' => (vec![KeyCode::KEY_A], false),
        'b' => (vec![KeyCode::KEY_B], false),
        'c' => (vec![KeyCode::KEY_C], false),
        'd' => (vec![KeyCode::KEY_D], false),
        'e' => (vec![KeyCode::KEY_E], false),
        'f' => (vec![KeyCode::KEY_F], false),
        'g' => (vec![KeyCode::KEY_G], false),
        'h' => (vec![KeyCode::KEY_H], false),
        'i' => (vec![KeyCode::KEY_I], false),
        'j' => (vec![KeyCode::KEY_J], false),
        'k' => (vec![KeyCode::KEY_K], false),
        'l' => (vec![KeyCode::KEY_L], false),
        'm' => (vec![KeyCode::KEY_M], false),
        'n' => (vec![KeyCode::KEY_N], false),
        'o' => (vec![KeyCode::KEY_O], false),
        'p' => (vec![KeyCode::KEY_P], false),
        'q' => (vec![KeyCode::KEY_Q], false),
        'r' => (vec![KeyCode::KEY_R], false),
        's' => (vec![KeyCode::KEY_S], false),
        't' => (vec![KeyCode::KEY_T], false),
        'u' => (vec![KeyCode::KEY_U], false),
        'v' => (vec![KeyCode::KEY_V], false),
        'w' => (vec![KeyCode::KEY_W], false),
        'x' => (vec![KeyCode::KEY_X], false),
        'y' => (vec![KeyCode::KEY_Y], false),
        'z' => (vec![KeyCode::KEY_Z], false),
        'A' => (vec![KeyCode::KEY_A], true),
        'B' => (vec![KeyCode::KEY_B], true),
        'C' => (vec![KeyCode::KEY_C], true),
        'D' => (vec![KeyCode::KEY_D], true),
        'E' => (vec![KeyCode::KEY_E], true),
        'F' => (vec![KeyCode::KEY_F], true),
        'G' => (vec![KeyCode::KEY_G], true),
        'H' => (vec![KeyCode::KEY_H], true),
        'I' => (vec![KeyCode::KEY_I], true),
        'J' => (vec![KeyCode::KEY_J], true),
        'K' => (vec![KeyCode::KEY_K], true),
        'L' => (vec![KeyCode::KEY_L], true),
        'M' => (vec![KeyCode::KEY_M], true),
        'N' => (vec![KeyCode::KEY_N], true),
        'O' => (vec![KeyCode::KEY_O], true),
        'P' => (vec![KeyCode::KEY_P], true),
        'Q' => (vec![KeyCode::KEY_Q], true),
        'R' => (vec![KeyCode::KEY_R], true),
        'S' => (vec![KeyCode::KEY_S], true),
        'T' => (vec![KeyCode::KEY_T], true),
        'U' => (vec![KeyCode::KEY_U], true),
        'V' => (vec![KeyCode::KEY_V], true),
        'W' => (vec![KeyCode::KEY_W], true),
        'X' => (vec![KeyCode::KEY_X], true),
        'Y' => (vec![KeyCode::KEY_Y], true),
        'Z' => (vec![KeyCode::KEY_Z], true),
        '0' => (vec![KeyCode::KEY_0], false),
        '1' => (vec![KeyCode::KEY_1], false),
        '2' => (vec![KeyCode::KEY_2], false),
        '3' => (vec![KeyCode::KEY_3], false),
        '4' => (vec![KeyCode::KEY_4], false),
        '5' => (vec![KeyCode::KEY_5], false),
        '6' => (vec![KeyCode::KEY_6], false),
        '7' => (vec![KeyCode::KEY_7], false),
        '8' => (vec![KeyCode::KEY_8], false),
        '9' => (vec![KeyCode::KEY_9], false),
        ' ' => (vec![KeyCode::KEY_SPACE], false),
        '.' => (vec![KeyCode::KEY_DOT], false),
        ',' => (vec![KeyCode::KEY_COMMA], false),
        ';' => (vec![KeyCode::KEY_SEMICOLON], false),
        '/' => (vec![KeyCode::KEY_SLASH], false),
        '[' => (vec![KeyCode::KEY_LEFTBRACE], false),
        ']' => (vec![KeyCode::KEY_RIGHTBRACE], false),
        '\\' => (vec![KeyCode::KEY_BACKSLASH], false),
        '-' => (vec![KeyCode::KEY_MINUS], false),
        '=' => (vec![KeyCode::KEY_EQUAL], false),
        '#' => (vec![KeyCode::KEY_3], true),
        '!' => (vec![KeyCode::KEY_1], true),
        '@' => (vec![KeyCode::KEY_2], true),
        '$' => (vec![KeyCode::KEY_4], true),
        '%' => (vec![KeyCode::KEY_5], true),
        '^' => (vec![KeyCode::KEY_6], true),
        '&' => (vec![KeyCode::KEY_7], true),
        '*' => (vec![KeyCode::KEY_8], true),
        '(' => (vec![KeyCode::KEY_9], true),
        ')' => (vec![KeyCode::KEY_0], true),
        '_' => (vec![KeyCode::KEY_MINUS], true),
        '+' => (vec![KeyCode::KEY_EQUAL], true),
        '{' => (vec![KeyCode::KEY_LEFTBRACE], true),
        '}' => (vec![KeyCode::KEY_RIGHTBRACE], true),
        '|' => (vec![KeyCode::KEY_BACKSLASH], true),
        ':' => (vec![KeyCode::KEY_SEMICOLON], true),
        '"' => (vec![KeyCode::KEY_APOSTROPHE], true),
        '<' => (vec![KeyCode::KEY_COMMA], true),
        '>' => (vec![KeyCode::KEY_DOT], true),
        '?' => (vec![KeyCode::KEY_SLASH], true),
        '~' => (vec![KeyCode::KEY_GRAVE], true),
        '`' => (vec![KeyCode::KEY_GRAVE], false),
        '\'' => (vec![KeyCode::KEY_APOSTROPHE], false),
        '\n' => (vec![KeyCode::KEY_ENTER], false),
        '\t' => (vec![KeyCode::KEY_TAB], false),
        '“' | '”' => (vec![KeyCode::KEY_APOSTROPHE], true),
        '‘' | '’' => (vec![KeyCode::KEY_APOSTROPHE], false),
        '—' | '–' => (vec![KeyCode::KEY_MINUS], false),
        '…' => (vec![KeyCode::KEY_DOT, KeyCode::KEY_DOT, KeyCode::KEY_DOT], false),
        _ => (vec![KeyCode::KEY_SPACE], false),
    }
}

#[cfg(target_os = "windows")]
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
        '“' | '”' => (vec![VK_OEM_7], true),
        '‘' | '’' => (vec![VK_OEM_7], false),
        '—' | '–' => (vec![VK_OEM_MINUS], false),
        '…' => (vec![VK_OEM_PERIOD, VK_OEM_PERIOD, VK_OEM_PERIOD], false),
        _ => (vec![VK_SPACE], false),
    }
}

pub fn type_text_hardware(
    text: &str, 
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
    #[allow(unused_variables)]
    virtual_keyboard: Arc<Mutex<Option<crate::VirtualKeyboardHandle>>>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    let hold_duration = Duration::from_millis(key_press_duration_ms);
    
    crate::log_info!("⌨️  [Hardware Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)", text, interval_ms, key_press_duration_ms);

    #[cfg(target_os = "linux")]
    {
        let mut keyboard_lock = virtual_keyboard.lock().unwrap();
        if let Some(device) = keyboard_lock.as_mut() {
            for ch in text.chars() {
                let (key_codes, needs_shift) = char_to_keys(ch);
                if needs_shift { device.emit(&[KeyEvent::new(KeyCode::KEY_LEFTSHIFT, 1).into()])?; }
                for key in &key_codes { device.emit(&[KeyEvent::new(*key, 1).into()])?; }
                thread::sleep(hold_duration);
                for key in &key_codes { device.emit(&[KeyEvent::new(*key, 0).into()])?; }
                if needs_shift { device.emit(&[KeyEvent::new(KeyCode::KEY_LEFTSHIFT, 0).into()])?; }
                device.emit(&[SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0).into()])?;
                if interval_ms > 0 { thread::sleep(Duration::from_millis(interval_ms)); }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        for ch in text.chars() {
            let (vk_codes, needs_shift) = char_to_vks(ch);
            unsafe {
                if needs_shift { emit_vk(VK_SHIFT, true); }
                for vk in &vk_codes { emit_vk(*vk, true); }
                thread::sleep(hold_duration);
                for vk in &vk_codes { emit_vk(*vk, false); }
                if needs_shift { emit_vk(VK_SHIFT, false); }
            }
            if interval_ms > 0 { thread::sleep(Duration::from_millis(interval_ms)); }
        }
    }

    crate::log_info!("✅ Hardware typing complete");
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn emit_vk(vk: VIRTUAL_KEY, is_down: bool) {
    let mut input = INPUT::default();
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: if is_down { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP },
        time: 0,
        dwExtraInfo: 0,
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::log_info!("📋 Attempting to copy to clipboard ({} chars)...", text.len());
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    crate::log_info!("✅ Copied to clipboard successfully");
    Ok(())
}
