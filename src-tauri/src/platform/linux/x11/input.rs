use std::thread;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xtest::ConnectionExt as XTestExt;
use x11rb::rust_connection::RustConnection;

const XK_SHIFT_L: u32 = 0xFFE1;
const XK_SHIFT_R: u32 = 0xFFE2;
const XK_CONTROL_L: u32 = 0xFFE3;
const XK_CONTROL_R: u32 = 0xFFE4;
const XK_INSERT: u32 = 0xFF63;
const XK_RETURN: u32 = 0xFF0D;
const XK_TAB: u32 = 0xFF09;

#[derive(Clone, Copy)]
struct ResolvedKey {
    keycode: u8,
    needs_shift: bool,
}

struct KeyboardMap {
    keysyms_per_keycode: usize,
    min_keycode: u8,
    keysyms: Vec<u32>,
    shift_keycode: u8,
    ctrl_keycode: u8,
}

fn char_to_keysym(character: char) -> Option<u32> {
    match character {
        '\n' => Some(XK_RETURN),
        '\t' => Some(XK_TAB),
        _ if character.is_ascii() => Some(character as u32),
        _ => Some(0x0100_0000 + character as u32),
    }
}

fn load_keyboard_map(
    connection: &RustConnection,
) -> Result<KeyboardMap, Box<dyn std::error::Error + Send + Sync>> {
    let setup = connection.setup();
    let min_keycode = setup.min_keycode;
    let keycode_count = setup.max_keycode - setup.min_keycode + 1;
    let reply = connection
        .get_keyboard_mapping(min_keycode, keycode_count)?
        .reply()?;

    let keysyms_per_keycode = reply.keysyms_per_keycode as usize;
    let keysyms = reply.keysyms;

    let shift_keycode = resolve_keysym_keycode_raw(
        min_keycode,
        keysyms_per_keycode,
        &keysyms,
        &[XK_SHIFT_L, XK_SHIFT_R],
    )
    .unwrap_or(50);

    let ctrl_keycode = resolve_keysym_keycode_raw(
        min_keycode,
        keysyms_per_keycode,
        &keysyms,
        &[XK_CONTROL_L, XK_CONTROL_R],
    )
    .unwrap_or(37);

    Ok(KeyboardMap {
        keysyms_per_keycode,
        min_keycode,
        keysyms,
        shift_keycode,
        ctrl_keycode,
    })
}

fn resolve_keysym_keycode_raw(
    min_keycode: u8,
    keysyms_per_keycode: usize,
    keysyms: &[u32],
    targets: &[u32],
) -> Option<u8> {
    for (index, chunk) in keysyms.chunks(keysyms_per_keycode).enumerate() {
        if chunk.iter().any(|keysym| targets.contains(keysym)) {
            return Some(min_keycode.saturating_add(index as u8));
        }
    }
    None
}

fn resolve_keysym_keycode(keyboard_map: &KeyboardMap, target: u32) -> Option<ResolvedKey> {
    for (index, chunk) in keyboard_map
        .keysyms
        .chunks(keyboard_map.keysyms_per_keycode)
        .enumerate()
    {
        for (column, keysym) in chunk.iter().enumerate() {
            if *keysym == target {
                return Some(ResolvedKey {
                    keycode: keyboard_map.min_keycode.saturating_add(index as u8),
                    needs_shift: column % 2 == 1,
                });
            }
        }
    }

    None
}

fn resolve_text_keys(
    text: &str,
    keyboard_map: &KeyboardMap,
) -> Result<Vec<ResolvedKey>, Vec<char>> {
    let mut resolved = Vec::with_capacity(text.chars().count());
    let mut unsupported = Vec::new();

    for character in text.chars() {
        let Some(keysym) = char_to_keysym(character) else {
            unsupported.push(character);
            continue;
        };

        let Some(key) = resolve_keysym_keycode(keyboard_map, keysym) else {
            unsupported.push(character);
            continue;
        };

        resolved.push(key);
    }

    if unsupported.is_empty() {
        Ok(resolved)
    } else {
        Err(unsupported)
    }
}

fn send_key_event(
    connection: &RustConnection,
    keycode: u8,
    press: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event_type = if press {
        x11rb::protocol::xproto::KEY_PRESS_EVENT
    } else {
        x11rb::protocol::xproto::KEY_RELEASE_EVENT
    };

    connection.xtest_fake_input(event_type, keycode, 0, x11rb::NONE, 0, 0, 0)?;
    Ok(())
}

fn paste_via_clipboard_shortcut(
    connection: &RustConnection,
    keyboard_map: &KeyboardMap,
    shortcut: crate::config::PasteShortcut,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Safety clear: release modifiers first to clear any latent modifier state
    send_key_event(connection, keyboard_map.ctrl_keycode, false)?;
    send_key_event(connection, keyboard_map.shift_keycode, false)?;
    connection.flush()?;
    thread::sleep(Duration::from_millis(10));

    match shortcut {
        crate::config::PasteShortcut::ShiftInsert => {
            let insert_key = resolve_keysym_keycode(keyboard_map, XK_INSERT)
                .ok_or_else(|| "Failed to resolve keycode for Insert".to_string())?;

            send_key_event(connection, keyboard_map.shift_keycode, true)?;
            connection.flush()?;
            thread::sleep(Duration::from_millis(50));

            send_key_event(connection, insert_key.keycode, true)?;
            send_key_event(connection, insert_key.keycode, false)?;
            connection.flush()?;
            thread::sleep(Duration::from_millis(50));

            send_key_event(connection, keyboard_map.shift_keycode, false)?;
            send_key_event(connection, keyboard_map.shift_keycode, false)?;
            connection.flush()?;
        }
        crate::config::PasteShortcut::CtrlV => {
            let v_key = resolve_keysym_keycode(keyboard_map, 'v' as u32)
                .ok_or_else(|| "Failed to resolve keycode for 'v'".to_string())?;

            send_key_event(connection, keyboard_map.ctrl_keycode, true)?;
            connection.flush()?;
            thread::sleep(Duration::from_millis(50));

            send_key_event(connection, v_key.keycode, true)?;
            send_key_event(connection, v_key.keycode, false)?;
            connection.flush()?;
            thread::sleep(Duration::from_millis(50));

            send_key_event(connection, keyboard_map.ctrl_keycode, false)?;
            send_key_event(connection, keyboard_map.ctrl_keycode, false)?;
            connection.flush()?;
        }
        crate::config::PasteShortcut::CtrlShiftV => {
            let v_key = resolve_keysym_keycode(keyboard_map, 'v' as u32)
                .ok_or_else(|| "Failed to resolve keycode for 'v'".to_string())?;

            send_key_event(connection, keyboard_map.ctrl_keycode, true)?;
            send_key_event(connection, keyboard_map.shift_keycode, true)?;
            connection.flush()?;
            thread::sleep(Duration::from_millis(50));

            send_key_event(connection, v_key.keycode, true)?;
            send_key_event(connection, v_key.keycode, false)?;
            connection.flush()?;
            thread::sleep(Duration::from_millis(50));

            send_key_event(connection, keyboard_map.shift_keycode, false)?;
            send_key_event(connection, keyboard_map.shift_keycode, false)?;
            send_key_event(connection, keyboard_map.ctrl_keycode, false)?;
            send_key_event(connection, keyboard_map.ctrl_keycode, false)?;
            connection.flush()?;
        }
    }
    Ok(())
}

pub fn send_paste_shortcut(
    shortcut: crate::config::PasteShortcut,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::log_info!("[X11] send_paste_shortcut: connecting to X11 display...");
    let (connection, _screen_num) = RustConnection::connect(None)?;
    let keyboard_map = load_keyboard_map(&connection)?;
    crate::log_info!(
        "[X11] send_paste_shortcut: loaded keyboard map, sending {:?} via XTest",
        shortcut
    );
    let result = paste_via_clipboard_shortcut(&connection, &keyboard_map, shortcut);
    match &result {
        Ok(()) => crate::log_info!("[X11] send_paste_shortcut: completed successfully"),
        Err(e) => crate::log_warn!("[X11] send_paste_shortcut: failed: {}", e),
    }
    result
}

pub fn type_text_hardware(
    text: &str,
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
    session_state: &std::sync::Arc<std::sync::Mutex<crate::app::state::SessionState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    crate::log_info!(
        "[X11 Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
        text,
        interval_ms,
        key_press_duration_ms
    );

    let (connection, _screen_num) = RustConnection::connect(None)?;
    let keyboard_map = load_keyboard_map(&connection)?;

    let resolved_keys = match resolve_text_keys(text, &keyboard_map) {
        Ok(keys) => keys,
        Err(unsupported_characters) => {
            let unsupported = unsupported_characters
                .iter()
                .map(|character| format!("'{}'", character))
                .collect::<Vec<String>>()
                .join(", ");
            crate::log_warn!(
                "[X11 Engine] Unmappable characters detected ({}). Falling back to clipboard paste.",
                unsupported
            );
            crate::typing::copy_to_clipboard(text)?;
            paste_via_clipboard_shortcut(
                &connection,
                &keyboard_map,
                crate::config::PasteShortcut::ShiftInsert,
            )?;
            crate::log_info!("X11 Clipboard fallback paste complete");
            return Ok(());
        }
    };

    for key in resolved_keys {
        if *session_state.lock().unwrap() != crate::app::state::SessionState::Typing {
            crate::log_info!("[X11 Engine] Typing aborted: session was cancelled");
            break;
        }

        if key.needs_shift {
            send_key_event(&connection, keyboard_map.shift_keycode, true)?;
        }

        send_key_event(&connection, key.keycode, true)?;
        connection.flush()?;
        thread::sleep(hold_duration);

        send_key_event(&connection, key.keycode, false)?;

        if key.needs_shift {
            send_key_event(&connection, keyboard_map.shift_keycode, false)?;
        }

        connection.flush()?;
        if interval_ms > 0 {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }

    crate::log_info!("X11 Hardware typing complete");
    Ok(())
}

fn parse_x11_keysym(token: &str) -> Result<u32, String> {
    let lower = token.trim().to_lowercase();
    let stripped = lower.strip_prefix("key").unwrap_or(&lower);
    let stripped_digit = stripped.strip_prefix("digit").unwrap_or(stripped);
    let stripped_num = stripped_digit.strip_prefix("num").unwrap_or(stripped_digit);

    if stripped_num.len() == 1 {
        let ch = stripped_num.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Ok(ch.to_ascii_lowercase() as u32);
        }
        if ch.is_ascii_digit() {
            return Ok(ch as u32);
        }
        match ch {
            ' ' => return Ok(0x0020),
            '.' => return Ok(0x002e),
            ',' => return Ok(0x002c),
            ';' => return Ok(0x003b),
            '/' => return Ok(0x002f),
            '[' => return Ok(0x005b),
            ']' => return Ok(0x005d),
            '\\' => return Ok(0x005c),
            '-' => return Ok(0x002d),
            '=' => return Ok(0x003d),
            '`' => return Ok(0x0060),
            '\'' => return Ok(0x0027),
            _ => {}
        }
    }

    match lower.as_str() {
        "ctrl" | "control" | "lctrl" | "rctrl" => Ok(XK_CONTROL_L),
        "shift" | "lshift" | "rshift" => Ok(XK_SHIFT_L),
        "alt" | "menu" | "lalt" | "ralt" => Ok(0xFFE9),
        "super" | "win" | "cmd" | "meta" => Ok(0xFFEB),
        "space" => Ok(0x0020),
        "enter" | "return" => Ok(XK_RETURN),
        "tab" => Ok(XK_TAB),
        "esc" | "escape" => Ok(0xFF1B),
        "backspace" => Ok(0xFF08),
        "delete" | "del" => Ok(0xFFFF),
        "insert" | "ins" => Ok(XK_INSERT),
        "home" => Ok(0xFF50),
        "end" => Ok(0xFF57),
        "pageup" | "pgup" => Ok(0xFF55),
        "pagedown" | "pgdn" => Ok(0xFF56),
        "up" | "arrowup" => Ok(0xFF52),
        "down" | "arrowdown" => Ok(0xFF54),
        "left" | "arrowleft" => Ok(0xFF51),
        "right" | "arrowright" => Ok(0xFF53),
        "f1" => Ok(0xFFBE),
        "f2" => Ok(0xFFBF),
        "f3" => Ok(0xFFC0),
        "f4" => Ok(0xFFC1),
        "f5" => Ok(0xFFC2),
        "f6" => Ok(0xFFC3),
        "f7" => Ok(0xFFC4),
        "f8" => Ok(0xFFC5),
        "f9" => Ok(0xFFC6),
        "f10" => Ok(0xFFC7),
        "f11" => Ok(0xFFC8),
        "f12" => Ok(0xFFC9),
        _ => Err(format!(
            "Unrecognized key token '{token}'. Expected a valid key (e.g. F1-F12, Ctrl, Alt, Shift, Super, A-Z, 0-9, Space, Enter, Tab, Escape, etc.)"
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

    let (connection, _screen_num) = RustConnection::connect(None)?;
    let keyboard_map = load_keyboard_map(&connection)?;

    let mut modifier_keycodes = Vec::new();
    let mut main_keycodes = Vec::new();

    for part in parts {
        let keysym = parse_x11_keysym(part)?;
        if keysym == XK_CONTROL_L || keysym == XK_CONTROL_R {
            if !modifier_keycodes.contains(&keyboard_map.ctrl_keycode) {
                modifier_keycodes.push(keyboard_map.ctrl_keycode);
            }
        } else if keysym == XK_SHIFT_L || keysym == XK_SHIFT_R {
            if !modifier_keycodes.contains(&keyboard_map.shift_keycode) {
                modifier_keycodes.push(keyboard_map.shift_keycode);
            }
        } else {
            let resolved = resolve_keysym_keycode(&keyboard_map, keysym)
                .ok_or_else(|| format!("Failed to resolve X11 keycode for keysym {:#x}", keysym))?;
            main_keycodes.push(resolved.keycode);
        }
    }

    crate::log_info!(
        "[X11] Sending key combination '{}' (modifiers: {}, keys: {}, hold: {}ms)",
        combination,
        modifier_keycodes.len(),
        main_keycodes.len(),
        hold_duration_ms
    );

    // Release latent modifiers first
    send_key_event(&connection, keyboard_map.ctrl_keycode, false)?;
    send_key_event(&connection, keyboard_map.shift_keycode, false)?;
    connection.flush()?;
    thread::sleep(Duration::from_millis(10));

    for &mod_code in &modifier_keycodes {
        send_key_event(&connection, mod_code, true)?;
    }
    connection.flush()?;
    thread::sleep(Duration::from_millis(10));

    for &code in &main_keycodes {
        send_key_event(&connection, code, true)?;
    }
    connection.flush()?;

    thread::sleep(hold_duration);

    for &code in main_keycodes.iter().rev() {
        send_key_event(&connection, code, false)?;
    }
    connection.flush()?;
    thread::sleep(Duration::from_millis(10));

    for &mod_code in modifier_keycodes.iter().rev() {
        send_key_event(&connection, mod_code, false)?;
    }
    connection.flush()?;

    crate::log_info!("[X11] Key combination complete");
    Ok(())
}

pub fn send_key_down(key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (connection, _screen_num) = RustConnection::connect(None)?;
    let keyboard_map = load_keyboard_map(&connection)?;
    let keysym = parse_x11_keysym(key)?;
    let keycode = if keysym == XK_CONTROL_L || keysym == XK_CONTROL_R {
        keyboard_map.ctrl_keycode
    } else if keysym == XK_SHIFT_L || keysym == XK_SHIFT_R {
        keyboard_map.shift_keycode
    } else {
        let resolved = resolve_keysym_keycode(&keyboard_map, keysym)
            .ok_or_else(|| format!("Failed to resolve X11 keycode for keysym {:#x}", keysym))?;
        resolved.keycode
    };
    crate::log_info!("[X11] Sending KeyDown: '{}' (keycode: {})", key, keycode);
    send_key_event(&connection, keycode, true)?;
    connection.flush()?;
    Ok(())
}

pub fn send_key_up(key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (connection, _screen_num) = RustConnection::connect(None)?;
    let keyboard_map = load_keyboard_map(&connection)?;
    let keysym = parse_x11_keysym(key)?;
    let keycode = if keysym == XK_CONTROL_L || keysym == XK_CONTROL_R {
        keyboard_map.ctrl_keycode
    } else if keysym == XK_SHIFT_L || keysym == XK_SHIFT_R {
        keyboard_map.shift_keycode
    } else {
        let resolved = resolve_keysym_keycode(&keyboard_map, keysym)
            .ok_or_else(|| format!("Failed to resolve X11 keycode for keysym {:#x}", keysym))?;
        resolved.keycode
    };
    crate::log_info!("[X11] Sending KeyUp: '{}' (keycode: {})", key, keycode);
    send_key_event(&connection, keycode, false)?;
    connection.flush()?;
    Ok(())
}
