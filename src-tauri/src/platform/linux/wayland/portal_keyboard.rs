//! Sends keyboard input through the RemoteDesktop portal session: typed
//! text, paste shortcuts, key combinations and single key presses. The
//! session itself (creation, reconnection, the request loop) lives in
//! `input`, which calls these.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ashpd::desktop::remote_desktop::{KeyState, RemoteDesktop};

use crate::app::state::SessionState;

const XK_SHIFT_L: i32 = 0xFFE1;
const XK_CONTROL_L: i32 = 0xFFE3;
const XK_ALT_L: i32 = 0xFFE9;
const XK_SUPER_L: i32 = 0xFFEB;
const XK_INSERT: i32 = 0xFF63;
const XK_V: i32 = 0x76;

pub(super) async fn send_key(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    keysym: i32,
    state: KeyState,
) -> Result<(), String> {
    remote_desktop
        .notify_keyboard_keysym(session, keysym, state)
        .await
        .map_err(|error| format!("Portal key event failed for keysym {keysym:#x}: {error}"))
}

async fn release_shift(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
) -> Result<(), String> {
    send_key(remote_desktop, session, XK_SHIFT_L, KeyState::Released).await
}

async fn press_shift(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
) -> Result<(), String> {
    send_key(remote_desktop, session, XK_SHIFT_L, KeyState::Pressed).await
}

fn needs_shift(ch: char) -> bool {
    matches!(
        ch,
        'A'..='Z'
            | '!'
            | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '*'
            | '('
            | ')'
            | '_'
            | '+'
            | '{'
            | '}'
            | '|'
            | ':'
            | '"'
            | '<'
            | '>'
            | '?'
            | '~'
    )
}

fn base_keysym_for_char(ch: char) -> u32 {
    match ch {
        'A'..='Z' => (ch as u32) + 32,
        '!' => '1' as u32,
        '@' => '2' as u32,
        '#' => '3' as u32,
        '$' => '4' as u32,
        '%' => '5' as u32,
        '^' => '6' as u32,
        '&' => '7' as u32,
        '*' => '8' as u32,
        '(' => '9' as u32,
        ')' => '0' as u32,
        '_' => '-' as u32,
        '+' => '=' as u32,
        '{' => '[' as u32,
        '}' => ']' as u32,
        '|' => '\\' as u32,
        ':' => ';' as u32,
        '"' => '\'' as u32,
        '<' => ',' as u32,
        '>' => '.' as u32,
        '?' => '/' as u32,
        '~' => '`' as u32,
        _ => ch as u32,
    }
}

pub(super) async fn send_text_over_portal(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    text: &str,
    interval_ms: u64,
    hold_ms: u64,
    session_state: &Arc<Mutex<SessionState>>,
) -> Result<(), String> {
    crate::log_info!(
        "[Wayland Portal Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
        text,
        interval_ms,
        hold_ms
    );

    release_shift(remote_desktop, session).await?;

    for ch in text.chars() {
        if *session_state.lock().unwrap() != SessionState::Typing {
            crate::log_info!("[Wayland Portal Engine] Typing aborted: session was cancelled");
            break;
        }

        if needs_shift(ch) {
            let base = base_keysym_for_char(ch);
            press_shift(remote_desktop, session).await?;
            send_key(remote_desktop, session, base as i32, KeyState::Pressed).await?;
            tokio::time::sleep(Duration::from_millis(hold_ms)).await;
            send_key(remote_desktop, session, base as i32, KeyState::Released).await?;
            release_shift(remote_desktop, session).await?;
        } else {
            let keysym = crate::platform::linux::keysym::keysym_for_char(ch);
            send_key(remote_desktop, session, keysym as i32, KeyState::Pressed).await?;
            tokio::time::sleep(Duration::from_millis(hold_ms)).await;
            send_key(remote_desktop, session, keysym as i32, KeyState::Released).await?;
        }

        if interval_ms > 0 {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    }

    release_shift(remote_desktop, session).await?;

    crate::log_info!("Wayland portal typing complete");
    Ok(())
}

pub(super) async fn send_paste_shortcut_over_portal(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    shortcut: crate::config::PasteShortcut,
) -> Result<(), String> {
    let hold = Duration::from_millis(50);
    match shortcut {
        crate::config::PasteShortcut::CommandV => {
            return Err("Command+V paste is only supported on macOS".to_string());
        }
        crate::config::PasteShortcut::ShiftInsert => {
            crate::log_info!(
                "[Wayland Portal] send_paste_shortcut_over_portal: starting Shift+Insert sequence"
            );
            send_key(remote_desktop, session, XK_SHIFT_L, KeyState::Pressed).await?;
            tokio::time::sleep(hold).await;
            send_key(remote_desktop, session, XK_INSERT, KeyState::Pressed).await?;
            tokio::time::sleep(hold).await;
            send_key(remote_desktop, session, XK_INSERT, KeyState::Released).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
            send_key(remote_desktop, session, XK_SHIFT_L, KeyState::Released).await?;
        }
        crate::config::PasteShortcut::CtrlV => {
            crate::log_info!(
                "[Wayland Portal] send_paste_shortcut_over_portal: starting Ctrl+V sequence"
            );
            send_key(remote_desktop, session, XK_CONTROL_L, KeyState::Pressed).await?;
            tokio::time::sleep(hold).await;
            send_key(remote_desktop, session, XK_V, KeyState::Pressed).await?;
            tokio::time::sleep(hold).await;
            send_key(remote_desktop, session, XK_V, KeyState::Released).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
            send_key(remote_desktop, session, XK_CONTROL_L, KeyState::Released).await?;
        }
        crate::config::PasteShortcut::CtrlShiftV => {
            crate::log_info!(
                "[Wayland Portal] send_paste_shortcut_over_portal: starting Ctrl+Shift+V sequence"
            );
            send_key(remote_desktop, session, XK_CONTROL_L, KeyState::Pressed).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
            send_key(remote_desktop, session, XK_SHIFT_L, KeyState::Pressed).await?;
            tokio::time::sleep(hold).await;
            send_key(remote_desktop, session, XK_V, KeyState::Pressed).await?;
            tokio::time::sleep(hold).await;
            send_key(remote_desktop, session, XK_V, KeyState::Released).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
            send_key(remote_desktop, session, XK_SHIFT_L, KeyState::Released).await?;
            tokio::time::sleep(Duration::from_millis(10)).await;
            send_key(remote_desktop, session, XK_CONTROL_L, KeyState::Released).await?;
        }
    }

    crate::log_info!("[Wayland Portal] send_paste_shortcut_over_portal: sequence complete");
    Ok(())
}

pub(super) fn parse_wayland_keysym(token: &str) -> Result<i32, String> {
    let lower = token.trim().to_lowercase();
    let stripped = lower.strip_prefix("key").unwrap_or(&lower);
    let stripped_digit = stripped.strip_prefix("digit").unwrap_or(stripped);
    let stripped_num = stripped_digit.strip_prefix("num").unwrap_or(stripped_digit);

    if stripped_num.len() == 1 {
        let ch = stripped_num.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Ok(ch.to_ascii_lowercase() as i32);
        }
        if ch.is_ascii_digit() {
            return Ok(ch as i32);
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
        "alt" | "menu" | "lalt" | "ralt" => Ok(XK_ALT_L),
        "super" | "win" | "cmd" | "meta" => Ok(XK_SUPER_L),
        "space" => Ok(0x0020),
        "enter" | "return" => Ok(0xFF0D),
        "tab" => Ok(0xFF09),
        "esc" | "escape" => Ok(0xFF1B),
        "backspace" => Ok(0xFF08),
        "delete" | "del" => Ok(0xFFFF),
        "insert" | "ins" => Ok(0xFF63),
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

pub(super) async fn send_key_combination_over_portal(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    combination: &str,
    hold_duration_ms: u64,
) -> Result<(), String> {
    let hold = Duration::from_millis(hold_duration_ms.max(20));
    let parts: Vec<&str> = combination
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("Empty key combination".into());
    }

    let mut modifier_keysyms = Vec::new();
    let mut main_keysyms = Vec::new();

    for part in parts {
        let keysym = parse_wayland_keysym(part)?;
        if keysym == XK_CONTROL_L
            || keysym == XK_SHIFT_L
            || keysym == XK_ALT_L
            || keysym == XK_SUPER_L
        {
            if !modifier_keysyms.contains(&keysym) {
                modifier_keysyms.push(keysym);
            }
        } else {
            main_keysyms.push(keysym);
        }
    }

    crate::log_info!(
        "[Wayland Portal] Sending key combination '{}' (modifiers: {}, keys: {}, hold: {}ms)",
        combination,
        modifier_keysyms.len(),
        main_keysyms.len(),
        hold_duration_ms
    );

    for &mod_sym in &modifier_keysyms {
        send_key(remote_desktop, session, mod_sym, KeyState::Pressed).await?;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    for &sym in &main_keysyms {
        send_key(remote_desktop, session, sym, KeyState::Pressed).await?;
    }

    tokio::time::sleep(hold).await;

    for &sym in main_keysyms.iter().rev() {
        send_key(remote_desktop, session, sym, KeyState::Released).await?;
    }

    tokio::time::sleep(Duration::from_millis(10)).await;

    for &mod_sym in modifier_keysyms.iter().rev() {
        send_key(remote_desktop, session, mod_sym, KeyState::Released).await?;
    }

    crate::log_info!("[Wayland Portal] Key combination complete");
    Ok(())
}
