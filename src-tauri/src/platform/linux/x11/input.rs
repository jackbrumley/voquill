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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let v_key = resolve_keysym_keycode(keyboard_map, 'v' as u32)
        .ok_or_else(|| "Failed to resolve keycode for 'v'".to_string())?;

    send_key_event(connection, keyboard_map.ctrl_keycode, true)?;
    send_key_event(connection, v_key.keycode, true)?;
    send_key_event(connection, v_key.keycode, false)?;
    send_key_event(connection, keyboard_map.ctrl_keycode, false)?;
    connection.flush()?;
    Ok(())
}

pub fn type_text_hardware(
    text: &str,
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    let hold_duration = Duration::from_millis(key_press_duration_ms);

    crate::log_info!(
        "⌨️  [X11 Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
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
                "⚠️ [X11 Engine] Unmappable characters detected ({}). Falling back to clipboard paste.",
                unsupported
            );
            crate::typing::copy_to_clipboard(text)?;
            paste_via_clipboard_shortcut(&connection, &keyboard_map)?;
            crate::log_info!("✅ X11 Clipboard fallback paste complete");
            return Ok(());
        }
    };

    for key in resolved_keys {
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

    crate::log_info!("✅ X11 Hardware typing complete");
    Ok(())
}
