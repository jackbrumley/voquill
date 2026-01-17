use evdev::{uinput::VirtualDevice, KeyCode, InputEvent, EventType};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;

fn char_to_keys(ch: char) -> (Vec<KeyCode>, bool) {
    match ch {
        // Lowercase letters
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

        // Uppercase letters
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

        // Numbers
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

        // Symbols and Punctuation (US Layout assumption for now)
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
        '#' => (vec![KeyCode::KEY_3], true), // Fixed duplicated mapping
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

        // Typographic Smart Characters & Common Replacements
        '“' | '”' => (vec![KeyCode::KEY_APOSTROPHE], true),  // Smart Double Quotes -> "
        '‘' | '’' => (vec![KeyCode::KEY_APOSTROPHE], false), // Smart Single Quotes -> '
        '—' | '–' => (vec![KeyCode::KEY_MINUS], false),      // Em/En Dash -> -
        '…' => (vec![KeyCode::KEY_DOT, KeyCode::KEY_DOT, KeyCode::KEY_DOT], false), // Ellipsis -> ...

        // Fallback for unknown chars
        _ => (vec![KeyCode::KEY_SPACE], false),
    }
}

pub fn type_text_hardware(
    text: &str, 
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
    virtual_keyboard: Arc<Mutex<Option<VirtualDevice>>>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    
    let mut keyboard_lock = virtual_keyboard.lock().unwrap();
    if keyboard_lock.is_none() {
        return Err("Virtual hardware keyboard not initialized".into());
    }
    
    let device = keyboard_lock.as_mut().unwrap();
    log_info!("⌨️  [Hardware Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)", text, interval_ms, key_press_duration_ms);
    
    // Hold each key for a specified duration to simulate physical reality
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    for ch in text.chars() {
        let (key_codes, needs_shift) = char_to_keys(ch);

        // 1. Press Shift if needed
        if needs_shift {
            device.emit(&[InputEvent::new(EventType::KEY.0, KeyCode::KEY_LEFTSHIFT.0, 1)])?;
        }

        // 2. Press actual keys
        for key in &key_codes {
            device.emit(&[InputEvent::new(EventType::KEY.0, key.0, 1)])?;
        }

        thread::sleep(hold_duration);

        // 3. Release actual keys
        for key in &key_codes {
            device.emit(&[InputEvent::new(EventType::KEY.0, key.0, 0)])?;
        }

        // 4. Release Shift if needed
        if needs_shift {
            device.emit(&[InputEvent::new(EventType::KEY.0, KeyCode::KEY_LEFTSHIFT.0, 0)])?;
        }
        
        device.emit(&[InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0)])?;

        // Interval between characters
        if interval_ms > 0 {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }
    
    log_info!("✅ Hardware typing complete");
    Ok(())
}

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log_info!("📋 Attempting to copy to clipboard ({} chars)...", text.len());
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    log_info!("✅ Copied to clipboard successfully");
    Ok(())
}

