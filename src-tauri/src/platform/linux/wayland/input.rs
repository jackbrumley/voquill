use std::time::Duration;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use ashpd::desktop::PersistMode;
use tauri::Manager;

use crate::AppState;

pub struct WaylandTypeRequest {
    pub text: String,
    pub interval_ms: u64,
    pub hold_ms: u64,
    pub response: tokio::sync::oneshot::Sender<Result<(), String>>,
}

pub type WaylandTypeSender = tokio::sync::mpsc::UnboundedSender<WaylandTypeRequest>;

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
            requested_restore_token.as_deref(),
            PersistMode::DoNot,
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

    let input_token = selected_devices
        .restore_token()
        .map(|token| token.to_string());

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<WaylandTypeRequest>();
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
        crate::log_info!("⌨️ Wayland input emulation session started");

        loop {
            tokio::select! {
                _ = &mut cancel_receiver => {
                    crate::log_info!("🛑 Wayland input session cancelled.");
                    break;
                }
                maybe_request = receiver.recv() => {
                    let Some(request) = maybe_request else {
                        break;
                    };
                    let result = send_text_over_portal(
                        &remote_desktop,
                        &session,
                        &request.text,
                        request.interval_ms,
                        request.hold_ms,
                    ).await;
                    let _ = request.response.send(result);
                }
            }
        }

        if let Err(error) = session.close().await {
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
    sender
        .send(WaylandTypeRequest {
            text: text.to_string(),
            interval_ms,
            hold_ms: key_press_duration_ms,
            response: response_sender,
        })
        .map_err(|_| "Wayland input emulation session is unavailable.".to_string())?;

    response_receiver
        .await
        .map_err(|_| "Wayland input emulation response channel closed unexpectedly.".to_string())?
}

async fn send_text_over_portal(
    remote_desktop: &RemoteDesktop<'_>,
    session: &ashpd::desktop::Session<'_, RemoteDesktop<'_>>,
    text: &str,
    interval_ms: u64,
    hold_ms: u64,
) -> Result<(), String> {
    crate::log_info!(
        "⌨️  [Wayland Portal Engine] Typing: '{}' (Speed: {}ms, Hold: {}ms)",
        text,
        interval_ms,
        hold_ms
    );

    for ch in text.chars() {
        let keysym = keysym_for_char(ch);
        remote_desktop
            .notify_keyboard_keysym(session, keysym as i32, KeyState::Pressed)
            .await
            .map_err(|error| format!("Portal key press failed for '{}': {error}", ch))?;

        tokio::time::sleep(Duration::from_millis(hold_ms)).await;

        remote_desktop
            .notify_keyboard_keysym(session, keysym as i32, KeyState::Released)
            .await
            .map_err(|error| format!("Portal key release failed for '{}': {error}", ch))?;

        if interval_ms > 0 {
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    }

    crate::log_info!("✅ Wayland portal typing complete");
    Ok(())
}

fn keysym_for_char(ch: char) -> u32 {
    match ch {
        '\n' | '\r' => 0xff0d,
        '\t' => 0xff09,
        '“' | '”' => '"' as u32,
        '‘' | '’' => '\'' as u32,
        '—' | '–' => '-' as u32,
        '…' => '.' as u32,
        _ => ch as u32,
    }
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
