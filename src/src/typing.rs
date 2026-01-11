use evdev::{uinput::VirtualDevice, Key, InputEvent, EventType};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

pub fn type_text_hardware(
    text: &str, 
    typing_speed_interval: f64,
    virtual_keyboard: Arc<Mutex<Option<VirtualDevice>>>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    
    let mut keyboard_lock = virtual_keyboard.lock().unwrap();
    if keyboard_lock.is_none() {
        return Err("Virtual hardware keyboard not initialized".into());
    }
    
    let device = keyboard_lock.as_mut().unwrap();
    println!("⌨️  [Hardware Engine] Typing: '{}'", text);
    
    // Hold each key for a short duration to simulate physical reality
    // Most systems ignore keys held for less than 15-20ms
    let hold_duration = Duration::from_millis(20);

    for ch in text.chars() {
        let (key_codes, needs_shift) = match ch {
            'a'..='z' => (vec![Key::new(Key::KEY_A.0 + (ch as u16 - 'a' as u16))], false),
            'A'..='Z' => (vec![Key::new(Key::KEY_A.0 + (ch as u16 - 'A' as u16))], true),
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
            ' ' => (vec![Key::KEY_SPACE], false),
            '.' => (vec![Key::KEY_DOT], false),
            ',' => (vec![Key::KEY_COMMA], false),
            '!' => (vec![Key::KEY_1], true),
            '?' => (vec![Key::KEY_SLASH], true),
            '\'' => (vec![Key::KEY_APOSTROPHE], false),
            '"' => (vec![Key::KEY_APOSTROPHE], true),
            '\n' => (vec![Key::KEY_ENTER], false),
            _ => (vec![Key::KEY_SPACE], false), // Fallback
        };

        // 1. Press Shift if needed
        if needs_shift {
            device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 1)])?;
        }

        // 2. Press actual keys
        for key in &key_codes {
            device.emit(&[InputEvent::new(EventType::KEY, key.0, 1)])?;
        }

        // Wait for hold duration
        thread::sleep(hold_duration);

        // 3. Release actual keys
        for key in &key_codes {
            device.emit(&[InputEvent::new(EventType::KEY, key.0, 0)])?;
        }

        // 4. Release Shift if needed
        if needs_shift {
            device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 0)])?;
        }

        // Interval between characters
        if interval_ms > 0 {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }
    
    println!("✅ Hardware typing complete");
    Ok(())
}
