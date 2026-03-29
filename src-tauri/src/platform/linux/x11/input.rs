use std::thread;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xtest::ConnectionExt as XTestExt;
use x11rb::rust_connection::RustConnection;

// Helper to map char to rudimentary keycode for X11 (Very basic implementation for demo purposes)
fn char_to_keycode(ch: char) -> (u8, bool) {
    match ch {
        'a'..='z' => ((ch as u8) - b'a' + 38, false),
        'A'..='Z' => ((ch as u8) - b'A' + 38, true),
        ' ' => (65, false),
        _ => (0, false), // Unknown
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
        "⌨️  [X11 Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
        text,
        interval_ms,
        key_press_duration_ms
    );

    let (conn, _screen_num) = RustConnection::connect(None)?;
    let shift_keycode = 50; // Example left shift

    for ch in text.chars() {
        let (keycode, needs_shift) = char_to_keycode(ch);
        if keycode == 0 {
            continue;
        } // Skip unknown

        if needs_shift {
            conn.xtest_fake_input(
                x11rb::protocol::xproto::KEY_PRESS_EVENT,
                shift_keycode,
                0,
                x11rb::NONE,
                0,
                0,
                0,
            )?;
        }

        conn.xtest_fake_input(
            x11rb::protocol::xproto::KEY_PRESS_EVENT,
            keycode,
            0,
            x11rb::NONE,
            0,
            0,
            0,
        )?;
        conn.flush()?;
        thread::sleep(hold_duration);

        conn.xtest_fake_input(
            x11rb::protocol::xproto::KEY_RELEASE_EVENT,
            keycode,
            0,
            x11rb::NONE,
            0,
            0,
            0,
        )?;

        if needs_shift {
            conn.xtest_fake_input(
                x11rb::protocol::xproto::KEY_RELEASE_EVENT,
                shift_keycode,
                0,
                x11rb::NONE,
                0,
                0,
                0,
            )?;
        }

        conn.flush()?;
        if interval_ms > 0 {
            thread::sleep(Duration::from_millis(interval_ms));
        }
    }

    crate::log_info!("✅ X11 Hardware typing complete");
    Ok(())
}
