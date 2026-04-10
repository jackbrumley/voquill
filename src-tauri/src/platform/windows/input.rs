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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    crate::log_info!(
        "⌨️  [Hardware Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
        text,
        interval_ms,
        key_press_duration_ms
    );

    for ch in text.chars() {
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

    crate::log_info!("✅ Hardware typing complete");
    Ok(())
}

unsafe fn emit_vk(vk: VIRTUAL_KEY, is_down: bool) {
    let mut input = INPUT::default();
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: if is_down {
            KEYBD_EVENT_FLAGS(0)
        } else {
            KEYEVENTF_KEYUP
        },
        time: 0,
        dwExtraInfo: 0,
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}
