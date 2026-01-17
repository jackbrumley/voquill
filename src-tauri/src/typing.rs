use evdev::{uinput::VirtualDevice, Key, InputEvent, EventType};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use arboard::Clipboard;

fn char_to_keys(ch: char) -> (Vec<Key>, bool) {
    match ch {
        // Lowercase letters
        'a' => (vec![Key::KEY_A], false),
        'b' => (vec![Key::KEY_B], false),
        'c' => (vec![Key::KEY_C], false),
        'd' => (vec![Key::KEY_D], false),
        'e' => (vec![Key::KEY_E], false),
        'f' => (vec![Key::KEY_F], false),
        'g' => (vec![Key::KEY_G], false),
        'h' => (vec![Key::KEY_H], false),
        'i' => (vec![Key::KEY_I], false),
        'j' => (vec![Key::KEY_J], false),
        'k' => (vec![Key::KEY_K], false),
        'l' => (vec![Key::KEY_L], false),
        'm' => (vec![Key::KEY_M], false),
        'n' => (vec![Key::KEY_N], false),
        'o' => (vec![Key::KEY_O], false),
        'p' => (vec![Key::KEY_P], false),
        'q' => (vec![Key::KEY_Q], false),
        'r' => (vec![Key::KEY_R], false),
        's' => (vec![Key::KEY_S], false),
        't' => (vec![Key::KEY_T], false),
        'u' => (vec![Key::KEY_U], false),
        'v' => (vec![Key::KEY_V], false),
        'w' => (vec![Key::KEY_W], false),
        'x' => (vec![Key::KEY_X], false),
        'y' => (vec![Key::KEY_Y], false),
        'z' => (vec![Key::KEY_Z], false),

        // Uppercase letters
        'A' => (vec![Key::KEY_A], true),
        'B' => (vec![Key::KEY_B], true),
        'C' => (vec![Key::KEY_C], true),
        'D' => (vec![Key::KEY_D], true),
        'E' => (vec![Key::KEY_E], true),
        'F' => (vec![Key::KEY_F], true),
        'G' => (vec![Key::KEY_G], true),
        'H' => (vec![Key::KEY_H], true),
        'I' => (vec![Key::KEY_I], true),
        'J' => (vec![Key::KEY_J], true),
        'K' => (vec![Key::KEY_K], true),
        'L' => (vec![Key::KEY_L], true),
        'M' => (vec![Key::KEY_M], true),
        'N' => (vec![Key::KEY_N], true),
        'O' => (vec![Key::KEY_O], true),
        'P' => (vec![Key::KEY_P], true),
        'Q' => (vec![Key::KEY_Q], true),
        'R' => (vec![Key::KEY_R], true),
        'S' => (vec![Key::KEY_S], true),
        'T' => (vec![Key::KEY_T], true),
        'U' => (vec![Key::KEY_U], true),
        'V' => (vec![Key::KEY_V], true),
        'W' => (vec![Key::KEY_W], true),
        'X' => (vec![Key::KEY_X], true),
        'Y' => (vec![Key::KEY_Y], true),
        'Z' => (vec![Key::KEY_Z], true),

        // Numbers
        '0' => (vec![Key::KEY_0], false),
        '1' => (vec![Key::KEY_1], false),
        '2' => (vec![Key::KEY_2], false),
        '3' => (vec![Key::KEY_3], false),
        '4' => (vec![Key::KEY_4], false),
        '5' => (vec![Key::KEY_5], false),
        '6' => (vec![Key::KEY_6], false),
        '7' => (vec![Key::KEY_7], false),
        '8' => (vec![Key::KEY_8], false),
        '9' => (vec![Key::KEY_9], false),

        // Symbols and Punctuation (US Layout assumption for now)
        ' ' => (vec![Key::KEY_SPACE], false),
        '.' => (vec![Key::KEY_DOT], false),
        ',' => (vec![Key::KEY_COMMA], false),
        ';' => (vec![Key::KEY_SEMICOLON], false),
        '/' => (vec![Key::KEY_SLASH], false),
        '[' => (vec![Key::KEY_LEFTBRACE], false),
        ']' => (vec![Key::KEY_RIGHTBRACE], false),
        '\\' => (vec![Key::KEY_BACKSLASH], false),
        '-' => (vec![Key::KEY_MINUS], false),
        '=' => (vec![Key::KEY_EQUAL], false),
        '#' => (vec![Key::KEY_3], true), // Fixed duplicated mapping
        '!' => (vec![Key::KEY_1], true),
        '@' => (vec![Key::KEY_2], true),
        '$' => (vec![Key::KEY_4], true),
        '%' => (vec![Key::KEY_5], true),
        '^' => (vec![Key::KEY_6], true),
        '&' => (vec![Key::KEY_7], true),
        '*' => (vec![Key::KEY_8], true),
        '(' => (vec![Key::KEY_9], true),
        ')' => (vec![Key::KEY_0], true),
        '_' => (vec![Key::KEY_MINUS], true),
        '+' => (vec![Key::KEY_EQUAL], true),
        '{' => (vec![Key::KEY_LEFTBRACE], true),
        '}' => (vec![Key::KEY_RIGHTBRACE], true),
        '|' => (vec![Key::KEY_BACKSLASH], true),
        ':' => (vec![Key::KEY_SEMICOLON], true),
        '"' => (vec![Key::KEY_APOSTROPHE], true),
        '<' => (vec![Key::KEY_COMMA], true),
        '>' => (vec![Key::KEY_DOT], true),
        '?' => (vec![Key::KEY_SLASH], true),
        '~' => (vec![Key::KEY_GRAVE], true),
        '`' => (vec![Key::KEY_GRAVE], false),
        '\'' => (vec![Key::KEY_APOSTROPHE], false),
        '\n' => (vec![Key::KEY_ENTER], false),
        '\t' => (vec![Key::KEY_TAB], false),

        // Typographic Smart Characters & Common Replacements
        '“' | '”' => (vec![Key::KEY_APOSTROPHE], true),  // Smart Double Quotes -> "
        '‘' | '’' => (vec![Key::KEY_APOSTROPHE], false), // Smart Single Quotes -> '
        '—' | '–' => (vec![Key::KEY_MINUS], false),      // Em/En Dash -> -
        '…' => (vec![Key::KEY_DOT, Key::KEY_DOT, Key::KEY_DOT], false), // Ellipsis -> ...

        // Fallback for unknown chars
        _ => (vec![Key::KEY_SPACE], false),
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
    println!("⌨️  [Hardware Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)", text, interval_ms, key_press_duration_ms);
    
    // Hold each key for a specified duration to simulate physical reality
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    for ch in text.chars() {
        let (key_codes, needs_shift) = char_to_keys(ch);

        // 1. Press Shift if needed
        if needs_shift {
            device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 1)])?;
        }

        // 2. Press actual keys
        for key in &key_codes {
            device.emit(&[InputEvent::new(EventType::KEY, key.0, 1)])?;
        }

        thread::sleep(hold_duration);

        // 3. Release actual keys
        for key in &key_codes {
            device.emit(&[InputEvent::new(EventType::KEY, key.0, 0)])?;
        }

        // 4. Release Shift if needed
        if needs_shift {
            device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 0)])?;
        }
        
        device.emit(&[InputEvent::new(EventType::SYNCHRONIZATION, 0, 0)])?;

        // Interval between characters
        if interval_ms > 0 {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }
    
    println!("✅ Hardware typing complete");
    Ok(())
}

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}

pub fn paste_text_hardware(
    text: &str,
    key_press_duration_ms: u64,
    virtual_keyboard: Arc<Mutex<Option<VirtualDevice>>>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📋 [Hardware Engine] Pasting via clipboard: '{}' (Hold: {}ms)", text, key_press_duration_ms);
    
    // 1. Set clipboard content
    copy_to_clipboard(text)?;

    let mut keyboard_lock = virtual_keyboard.lock().unwrap();
    if keyboard_lock.is_none() {
        return Err("Virtual hardware keyboard not initialized".into());
    }
    
    let device = keyboard_lock.as_mut().unwrap();
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    // 2. Emit Ctrl+V (or Cmd+V for Mac)
    // For Linux/Windows, we use Control + V
    // Note: On Linux, some apps might use Shift+Insert, but Ctrl+V is universal for text fields.
    
    // Press Control
    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTCTRL.0, 1)])?;
    
    // Press V
    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_V.0, 1)])?;
    
    thread::sleep(hold_duration);
    
    // Release V
    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_V.0, 0)])?;
    
    // Release Control
    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTCTRL.0, 0)])?;
    
    device.emit(&[InputEvent::new(EventType::SYNCHRONIZATION, 0, 0)])?;

    println!("✅ Hardware paste complete");
    Ok(())
}
