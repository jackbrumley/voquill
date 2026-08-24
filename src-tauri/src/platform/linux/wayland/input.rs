use std::sync::{Arc, Mutex};
use std::time::Duration;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use ashpd::desktop::PersistMode;
use tauri::Manager;

use crate::app::state::SessionState;
use crate::AppState;

const XK_SHIFT_L: i32 = 0xFFE1;
const XK_CONTROL_L: i32 = 0xFFE3;
const XK_ALT_L: i32 = 0xFFE9;
const XK_SUPER_L: i32 = 0xFFEB;
const XK_INSERT: i32 = 0xFF63;
const XK_V: i32 = 0x76;

pub struct WaylandTypeRequest {
    pub text: String,
    pub interval_ms: u64,
    pub hold_ms: u64,
    pub session_state: Arc<Mutex<SessionState>>,
    pub response: tokio::sync::oneshot::Sender<Result<(), String>>,
}

pub enum WaylandInputRequest {
    TypeText(WaylandTypeRequest),
    SendPasteShortcut {
        shortcut: crate::config::PasteShortcut,
        response: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    SendKeyCombination {
        combination: String,
        hold_duration_ms: u64,
        response: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    SendKeyDown {
        key: String,
        response: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    SendKeyUp {
        key: String,
        response: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
}

pub type WaylandInputSender = tokio::sync::mpsc::UnboundedSender<WaylandInputRequest>;

async fn create_portal_session(
    restore_token: Option<&str>,
) -> Result<
    (
        RemoteDesktop<'static>,
        ashpd::desktop::Session<'static, RemoteDesktop<'static>>,
        Option<String>,
    ),
    String,
> {
    let remote_desktop = RemoteDesktop::new()
        .await
        .map_err(|error| format!("Remote Desktop Portal not available: {error}"))?;
    let session = remote_desktop
        .create_session()
        .await
        .map_err(|error| format!("Failed to create remote desktop session: {error}"))?;

    let select_request = remote_desktop
        .select_devices(
            &session,
            DeviceType::Keyboard.into(),
            restore_token,
            PersistMode::ExplicitlyRevoked,
        )
        .await
        .map_err(|error| format!("Failed to select keyboard devices: {error}"))?;
    select_request
        .response()
        .map_err(|error| format!("Input device selection denied or cancelled: {error}"))?;

    let start_request = remote_desktop
        .start(&session, None)
        .await
        .map_err(|error| format!("Failed to start remote desktop session: {error}"))?;
    let selected_devices = start_request
        .response()
        .map_err(|error| format!("Input emulation request denied or cancelled: {error}"))?;

    let new_token = selected_devices
        .restore_token()
        .map(|token| token.to_string());

    Ok((remote_desktop, session, new_token))
}

async fn reconnect_portal_session(
    app_handle: &tauri::AppHandle,
) -> Result<
    (
        RemoteDesktop<'static>,
        ashpd::desktop::Session<'static, RemoteDesktop<'static>>,
    ),
    String,
> {
    let stored_token = {
        let state = app_handle.state::<AppState>();
        let config = state.config.lock().unwrap();
        config.input_token.clone()
    };

    let (remote_desktop, session, new_token) =
        create_portal_session(stored_token.as_deref()).await?;

    if let Some(ref token) = new_token {
        let state = app_handle.state::<AppState>();
        let mut config = state.config.lock().unwrap();
        config.input_token = Some(token.clone());
        let _ = crate::config::save_config(&config);
    }

    Ok((remote_desktop, session))
}

pub async fn establish_input_session(
    app_handle: &tauri::AppHandle,
    force_rebind: bool,
) -> Result<(), String> {
    teardown_input_session(app_handle).await;

    let requested_restore_token = {
        let state = app_handle.state::<AppState>();
        let mut config = state.config.lock().unwrap();
        if force_rebind {
            None
        } else {
            match config.input_token.clone() {
                Some(token) if is_valid_restore_token(&token) => Some(token),
                Some(token) => {
                    crate::log_warn!(
                        "Ignoring invalid stored input restore token '{}'; requesting fresh portal session",
                        token
                    );
                    config.input_token = None;
                    let _ = crate::config::save_config(&config);
                    None
                }
                None => None,
            }
        }
    };

    let (remote_desktop, session, input_token) =
        create_portal_session(requested_restore_token.as_deref()).await?;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<WaylandInputRequest>();
    let (cancel_sender, mut cancel_receiver) = tokio::sync::oneshot::channel::<()>();

    {
        let state = app_handle.state::<AppState>();
        {
            let mut config = state.config.lock().unwrap();
            config.input_token = input_token;
            let _ = crate::config::save_config(&config);
        }
        {
            let mut sender_lock = state.wayland_input_sender.lock().unwrap();
            *sender_lock = Some(sender);
        }
        {
            let mut cancel_lock = state.wayland_input_cancel.lock().unwrap();
            *cancel_lock = Some(cancel_sender);
        }
        {
            let mut ready_lock = state.wayland_input_ready.lock().unwrap();
            *ready_lock = true;
        }
    }

    let app_handle_for_task = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::log_info!("Wayland input emulation session started");

        let mut current_remote_desktop = remote_desktop;
        let mut current_session = session;

        loop {
            tokio::select! {
                _ = &mut cancel_receiver => {
                    crate::log_info!("Wayland input session cancelled.");
                    break;
                }
                maybe_request = receiver.recv() => {
                    let Some(request) = maybe_request else {
                        break;
                    };

                    match request {
                        WaylandInputRequest::TypeText(req) => {
                            let text = req.text;
                            let interval_ms = req.interval_ms;
                            let hold_ms = req.hold_ms;
                            let session_state = req.session_state;
                            let response = req.response;

                            let result = send_text_over_portal(
                                &current_remote_desktop,
                                &current_session,
                                &text,
                                interval_ms,
                                hold_ms,
                                &session_state,
                            ).await;

                            let result = match result {
                                Ok(()) => Ok(()),
                                Err(error) => {
                                    crate::log_warn!(
                                        "Wayland typing failed (session may have expired): {}. Attempting reconnect...",
                                        error
                                    );

                                    {
                                        let state = app_handle_for_task.state::<AppState>();
                                        *state.wayland_input_ready.lock().unwrap() = false;
                                    }

                                    let _ = current_session.close().await;

                                    match reconnect_portal_session(&app_handle_for_task).await {
                                        Ok((new_rd, new_sess)) => {
                                            current_remote_desktop = new_rd;
                                            current_session = new_sess;
                                            {
                                                let state = app_handle_for_task.state::<AppState>();
                                                *state.wayland_input_ready.lock().unwrap() = true;
                                            }
                                            crate::log_info!("Wayland input session reconnected. Retrying typing...");

                                            send_text_over_portal(
                                                &current_remote_desktop,
                                                &current_session,
                                                &text,
                                                interval_ms,
                                                hold_ms,
                                                &session_state,
                                            ).await
                                        }
                                        Err(reconnect_error) => {
                                            crate::log_warn!(
                                                "Failed to reconnect Wayland input session: {}",
                                                reconnect_error
                                            );
                                            let state = app_handle_for_task.state::<AppState>();
                                            *state.wayland_input_ready.lock().unwrap() = false;
                                            Err(format!(
                                                "Input session expired and reconnection failed: {}",
                                                reconnect_error
                                            ))
                                        }
                                    }
                                }
                            };

                            let _ = response.send(result);
                        }
                        WaylandInputRequest::SendPasteShortcut { shortcut, response } => {
                            crate::log_info!("[Event Loop] Received SendPasteShortcut request ({:?}), sending through portal...", shortcut);
                            let res = send_paste_shortcut_over_portal(
                                &current_remote_desktop,
                                &current_session,
                                shortcut,
                            ).await;
                            match &res {
                                Ok(()) => crate::log_info!("[Event Loop] SendPasteShortcut portal call succeeded"),
                                Err(e) => crate::log_warn!("[Event Loop] SendPasteShortcut portal call failed: {}", e),
                            }
                            let _ = response.send(res);
                        }
                        WaylandInputRequest::SendKeyCombination { combination, hold_duration_ms, response } => {
                            crate::log_info!("[Event Loop] Received SendKeyCombination request ('{}'), sending through portal...", combination);
                            let res = send_key_combination_over_portal(
                                &current_remote_desktop,
                                &current_session,
                                &combination,
                                hold_duration_ms,
                            ).await;
                            match &res {
                                Ok(()) => crate::log_info!("[Event Loop] SendKeyCombination portal call succeeded"),
                                Err(e) => crate::log_warn!("[Event Loop] SendKeyCombination portal call failed: {}", e),
                            }
                            let _ = response.send(res);
                        }
                        WaylandInputRequest::SendKeyDown { key, response } => {
                            crate::log_info!("[Event Loop] Received SendKeyDown request ('{}')", key);
                            let res = match parse_wayland_keysym(&key) {
                                Ok(sym) => send_key(&current_remote_desktop, &current_session, sym, KeyState::Pressed).await,
                                Err(e) => Err(e),
                            };
                            let _ = response.send(res);
                        }
                        WaylandInputRequest::SendKeyUp { key, response } => {
                            crate::log_info!("[Event Loop] Received SendKeyUp request ('{}')", key);
                            let res = match parse_wayland_keysym(&key) {
                                Ok(sym) => send_key(&current_remote_desktop, &current_session, sym, KeyState::Released).await,
                                Err(e) => Err(e),
                            };
                            let _ = response.send(res);
                        }
                    }
                }
            }
        }

        if let Err(error) = current_session.close().await {
            crate::log_warn!("Failed to close Wayland input session cleanly: {}", error);
        }

        let state = app_handle_for_task.state::<AppState>();
        {
            let mut sender_lock = state.wayland_input_sender.lock().unwrap();
            *sender_lock = None;
        }
        {
            let mut cancel_lock = state.wayland_input_cancel.lock().unwrap();
            *cancel_lock = None;
        }
        {
            let mut ready_lock = state.wayland_input_ready.lock().unwrap();
            *ready_lock = false;
        }
    });

    Ok(())
}

pub async fn teardown_input_session(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<AppState>();

    let cancel = {
        let mut cancel_lock = state.wayland_input_cancel.lock().unwrap();
        cancel_lock.take()
    };

    if let Some(cancel_sender) = cancel {
        let _ = cancel_sender.send(());
    }

    {
        let mut sender_lock = state.wayland_input_sender.lock().unwrap();
        *sender_lock = None;
    }
    {
        let mut ready_lock = state.wayland_input_ready.lock().unwrap();
        *ready_lock = false;
    }
}

pub async fn type_text_hardware(
    app_handle: &tauri::AppHandle,
    text: &str,
    typing_speed_interval: f64,
    key_press_duration_ms: u64,
) -> Result<(), String> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;

    let sender = {
        let state = app_handle.state::<AppState>();
        let sender_lock = state.wayland_input_sender.lock().unwrap();
        sender_lock.clone()
    }
    .ok_or_else(|| {
        "Wayland input emulation is not active. Complete input setup to enable Typewriter mode."
            .to_string()
    })?;

    let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
    let session_state = app_handle.state::<AppState>().session_state.clone();
    sender
        .send(WaylandInputRequest::TypeText(WaylandTypeRequest {
            text: text.to_string(),
            interval_ms,
            hold_ms: key_press_duration_ms,
            session_state,
            response: response_sender,
        }))
        .map_err(|_| "Wayland input emulation session is unavailable.".to_string())?;

    response_receiver
        .await
        .map_err(|_| "Wayland input emulation response channel closed unexpectedly.".to_string())?
}

pub async fn send_paste_shortcut(
    app_handle: &tauri::AppHandle,
    shortcut: crate::config::PasteShortcut,
) -> Result<(), String> {
    crate::log_info!("send_paste_shortcut: checking for active Wayland input session...");
    let sender = {
        let state = app_handle.state::<AppState>();
        let sender_lock = state.wayland_input_sender.lock().unwrap();
        sender_lock.clone()
    };

    let sender = match sender {
        Some(s) => {
            crate::log_info!("send_paste_shortcut: Wayland input session found, sending request");
            s
        }
        None => {
            let msg =
                "Wayland input emulation is not active. Complete input setup to enable paste."
                    .to_string();
            crate::log_warn!("send_paste_shortcut: FAILED - {}", msg);
            return Err(msg);
        }
    };

    let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
    if let Err(e) = sender.send(WaylandInputRequest::SendPasteShortcut {
        shortcut,
        response: response_sender,
    }) {
        let msg = "Wayland input emulation session is unavailable.".to_string();
        crate::log_warn!("send_paste_shortcut: channel send failed: {}", e);
        return Err(msg);
    }

    crate::log_info!("send_paste_shortcut: waiting for portal response...");
    match response_receiver.await {
        Ok(Ok(())) => {
            crate::log_info!("send_paste_shortcut: portal paste shortcut completed successfully");
            Ok(())
        }
        Ok(Err(e)) => {
            crate::log_warn!("send_paste_shortcut: portal paste shortcut failed: {}", e);
            Err(e)
        }
        Err(e) => {
            let msg = format!("send_paste_shortcut: response channel closed: {}", e);
            crate::log_warn!("{}", msg);
            Err(msg)
        }
    }
}

pub async fn send_key_combination(
    app_handle: &tauri::AppHandle,
    combination: &str,
    hold_duration_ms: u64,
) -> Result<(), String> {
    crate::log_info!(
        "send_key_combination: starting portal call for '{}'",
        combination
    );
    let sender = {
        let state = app_handle.state::<AppState>();
        let sender_lock = state.wayland_input_sender.lock().unwrap();
        sender_lock.clone()
    };

    let sender = match sender {
        Some(s) => s,
        None => {
            let msg =
                "Wayland input emulation is not active. Complete input setup to enable macro execution."
                    .to_string();
            crate::log_warn!("send_key_combination: FAILED - {}", msg);
            return Err(msg);
        }
    };

    let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
    if let Err(e) = sender.send(WaylandInputRequest::SendKeyCombination {
        combination: combination.to_string(),
        hold_duration_ms,
        response: response_sender,
    }) {
        let msg = "Wayland input emulation session is unavailable.".to_string();
        crate::log_warn!("send_key_combination: channel send failed: {}", e);
        return Err(msg);
    }

    match response_receiver.await {
        Ok(Ok(())) => {
            crate::log_info!("send_key_combination: portal key combination completed successfully");
            Ok(())
        }
        Ok(Err(e)) => {
            crate::log_warn!("send_key_combination: portal key combination failed: {}", e);
            Err(e)
        }
        Err(e) => {
            let msg = format!("send_key_combination: response channel closed: {}", e);
            crate::log_warn!("{}", msg);
            Err(msg)
        }
    }
}

pub async fn send_key_down(app_handle: &tauri::AppHandle, key: &str) -> Result<(), String> {
    crate::log_info!("send_key_down: starting portal call for '{}'", key);
    let sender = {
        let state = app_handle.state::<AppState>();
        let sender_lock = state.wayland_input_sender.lock().unwrap();
        sender_lock.clone()
    }
    .ok_or_else(|| {
        "Wayland input emulation is not active. Complete input setup to enable macro execution."
            .to_string()
    })?;

    let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
    if let Err(e) = sender.send(WaylandInputRequest::SendKeyDown {
        key: key.to_string(),
        response: response_sender,
    }) {
        let msg = "Wayland input emulation session is unavailable.".to_string();
        crate::log_warn!("send_key_down: channel send failed: {}", e);
        return Err(msg);
    }

    match response_receiver.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(format!("send_key_down: response channel closed: {}", e)),
    }
}

pub async fn send_key_up(app_handle: &tauri::AppHandle, key: &str) -> Result<(), String> {
    crate::log_info!("send_key_up: starting portal call for '{}'", key);
    let sender = {
        let state = app_handle.state::<AppState>();
        let sender_lock = state.wayland_input_sender.lock().unwrap();
        sender_lock.clone()
    }
    .ok_or_else(|| {
        "Wayland input emulation is not active. Complete input setup to enable macro execution."
            .to_string()
    })?;

    let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
    if let Err(e) = sender.send(WaylandInputRequest::SendKeyUp {
        key: key.to_string(),
        response: response_sender,
    }) {
        let msg = "Wayland input emulation session is unavailable.".to_string();
        crate::log_warn!("send_key_up: channel send failed: {}", e);
        return Err(msg);
    }

    match response_receiver.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(format!("send_key_up: response channel closed: {}", e)),
    }
}

async fn send_key(
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

async fn send_text_over_portal(
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
            let keysym = keysym_for_char(ch);
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

fn keysym_for_char(ch: char) -> u32 {
    match ch {
        '\n' | '\r' => 0xff0d,
        '\t' => 0xff09,
        _ => ch as u32,
    }
}

async fn send_paste_shortcut_over_portal(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    shortcut: crate::config::PasteShortcut,
) -> Result<(), String> {
    let hold = Duration::from_millis(50);
    match shortcut {
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

fn parse_wayland_keysym(token: &str) -> Result<i32, String> {
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

async fn send_key_combination_over_portal(
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

fn is_valid_restore_token(token: &str) -> bool {
    if token.len() != 36 {
        return false;
    }

    for (index, character) in token.chars().enumerate() {
        let is_hyphen_slot = matches!(index, 8 | 13 | 18 | 23);
        if is_hyphen_slot {
            if character != '-' {
                return false;
            }
        } else if !character.is_ascii_hexdigit() {
            return false;
        }
    }

    true
}
