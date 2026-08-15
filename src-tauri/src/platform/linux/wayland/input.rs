use std::sync::{Arc, Mutex};
use std::time::Duration;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use ashpd::desktop::PersistMode;
use tauri::Manager;

use crate::app::state::SessionState;
use crate::AppState;

const XK_SHIFT_L: i32 = 0xFFE1;
const XK_CONTROL_L: i32 = 0xFFE3;
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
    SendCtrlV {
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
                        WaylandInputRequest::SendCtrlV { response } => {
                            crate::log_info!("[Event Loop] Received SendCtrlV request, sending through portal...");
                            let res = send_ctrl_v_over_portal(
                                &current_remote_desktop,
                                &current_session,
                            ).await;
                            match &res {
                                Ok(()) => crate::log_info!("[Event Loop] SendCtrlV portal call succeeded"),
                                Err(e) => crate::log_warn!("[Event Loop] SendCtrlV portal call failed: {}", e),
                            }
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

pub async fn send_ctrl_v(app_handle: &tauri::AppHandle) -> Result<(), String> {
    crate::log_info!("send_ctrl_v: checking for active Wayland input session...");
    let sender = {
        let state = app_handle.state::<AppState>();
        let sender_lock = state.wayland_input_sender.lock().unwrap();
        sender_lock.clone()
    };

    let sender = match sender {
        Some(s) => {
            crate::log_info!("send_ctrl_v: Wayland input session found, sending request");
            s
        }
        None => {
            let msg =
                "Wayland input emulation is not active. Complete input setup to enable paste."
                    .to_string();
            crate::log_warn!("send_ctrl_v: FAILED - {}", msg);
            return Err(msg);
        }
    };

    let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
    if let Err(e) = sender.send(WaylandInputRequest::SendCtrlV {
        response: response_sender,
    }) {
        let msg = "Wayland input emulation session is unavailable.".to_string();
        crate::log_warn!("send_ctrl_v: channel send failed: {}", e);
        return Err(msg);
    }

    crate::log_info!("send_ctrl_v: waiting for portal response...");
    match response_receiver.await {
        Ok(Ok(())) => {
            crate::log_info!("send_ctrl_v: portal Ctrl+V completed successfully");
            Ok(())
        }
        Ok(Err(e)) => {
            crate::log_warn!("send_ctrl_v: portal Ctrl+V failed: {}", e);
            Err(e)
        }
        Err(e) => {
            let msg = format!("send_ctrl_v: response channel closed: {}", e);
            crate::log_warn!("{}", msg);
            Err(msg)
        }
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

async fn send_ctrl_v_over_portal(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
) -> Result<(), String> {
    crate::log_info!("[Wayland Portal] send_ctrl_v_over_portal: starting Ctrl+V sequence");

    let hold = Duration::from_millis(50);

    crate::log_info!(
        "[Wayland Portal] send_ctrl_v_over_portal: pressing Control_L (keysym 0xFFE3)"
    );
    send_key(remote_desktop, session, XK_CONTROL_L, KeyState::Pressed).await?;
    tokio::time::sleep(hold).await;

    crate::log_info!(
        "[Wayland Portal] send_ctrl_v_over_portal: pressing V (keysym 0x{:.2X})",
        XK_V
    );
    send_key(remote_desktop, session, XK_V, KeyState::Pressed).await?;
    tokio::time::sleep(hold).await;

    crate::log_info!("[Wayland Portal] send_ctrl_v_over_portal: releasing V");
    send_key(remote_desktop, session, XK_V, KeyState::Released).await?;
    tokio::time::sleep(Duration::from_millis(10)).await;

    crate::log_info!("[Wayland Portal] send_ctrl_v_over_portal: releasing Control_L");
    send_key(remote_desktop, session, XK_CONTROL_L, KeyState::Released).await?;

    crate::log_info!("[Wayland Portal] send_ctrl_v_over_portal: Ctrl+V sequence complete");
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
