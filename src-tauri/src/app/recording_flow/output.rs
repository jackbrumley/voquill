use crate::app::state::SessionState;
use crate::config::{Config, OutputMethod};
use crate::{history, typing};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub struct OutputPayload {
    pub output_text: String,
    pub diar_segments: Vec<crate::diarization::Segment>,
    pub raw_text: Option<String>,
    pub auto_submit: bool,
}

pub async fn deliver_output(
    app_handle: &AppHandle,
    session_state: &Arc<Mutex<SessionState>>,
    session_token: &Arc<AtomicBool>,
    config: &Arc<Mutex<Config>>,
    payload: OutputPayload,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let OutputPayload {
        output_text,
        diar_segments,
        raw_text,
        auto_submit,
    } = payload;
    let (
        typing_speed,
        hold_duration,
        output_method,
        copy_on_typewriter,
        paste_delay_before_ms,
        paste_delay_after_ms,
        paste_after_copy,
        paste_shortcut,
        history_limit,
    ) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.typing_speed_interval,
            config_guard.key_press_duration_ms,
            config_guard.output_method.clone(),
            config_guard.copy_on_typewriter,
            config_guard.paste_delay_before_ms,
            config_guard.paste_delay_after_ms,
            config_guard.paste_after_copy,
            config_guard.paste_shortcut,
            config_guard.history_limit,
        )
    };

    if output_text.trim().is_empty() {
        crate::log_info!("Transcription was empty, skipping output delivery.");
        return Ok(());
    }

    let segments_json = if diar_segments.is_empty() {
        None
    } else {
        serde_json::to_string(&diar_segments).ok()
    };

    let _ = history::add_history_item(
        &output_text,
        segments_json.as_deref(),
        Some(history_limit),
        raw_text.as_deref(),
    );
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.emit("history-updated", ());
    }

    {
        let mut session = session_state.lock().unwrap();
        *session = SessionState::Typing;
    }
    crate::app::status::emit_status_to_frontend("Typing").await;

    tokio::time::sleep(tokio::time::Duration::from_millis(paste_delay_before_ms)).await;

    if session_token.load(Ordering::SeqCst) {
        crate::log_info!("Session cancelled before output delivery; discarding output");
        return Ok(());
    }

    match output_method {
        OutputMethod::Typewriter => {
            if copy_on_typewriter {
                if let Err(error) = typing::copy_to_clipboard(&output_text) {
                    crate::log_info!("CLIPBOARD ERROR: {}", error);
                }
            }
            crate::log_info!("Forwarding text to hardware typing engine...");
            let state = app_handle.state::<crate::AppState>();
            if let Err(error) = state
                .display_backend
                .type_text_hardware(app_handle, &output_text, typing_speed, hold_duration)
                .await
            {
                crate::log_info!("TYPING ENGINE ERROR: {}", error);
            }
            if auto_submit {
                crate::log_info!("Auto-submitting with Enter...");
                if let Err(error) = state
                    .display_backend
                    .type_text_hardware(app_handle, "\n", typing_speed, hold_duration)
                    .await
                {
                    crate::log_info!("AUTO-SUBMIT ERROR: {}", error);
                }
            }
        }
        OutputMethod::Clipboard => {
            crate::log_info!("Copying text to clipboard (Clipboard Mode)...");
            let saved_clipboard = if paste_after_copy {
                typing::save_clipboard()
            } else {
                None
            };
            if let Err(error) = typing::copy_to_clipboard(&output_text) {
                crate::log_info!("CLIPBOARD ERROR: {}", error);
            }
            if paste_after_copy {
                crate::log_info!(
                    "Paste after copy: waiting {}ms before sending paste shortcut ({:?})...",
                    paste_delay_before_ms,
                    paste_shortcut
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(paste_delay_before_ms)).await;
                crate::log_info!("Calling display_backend.send_paste_shortcut()...");
                let state = app_handle.state::<crate::AppState>();
                match state
                    .display_backend
                    .send_paste_shortcut(app_handle, paste_shortcut)
                    .await
                {
                    Ok(()) => crate::log_info!("PASTE: send_paste_shortcut returned Ok"),
                    Err(error) => {
                        crate::log_warn!("PASTE ERROR: send_paste_shortcut failed: {}", error)
                    }
                }
                crate::log_info!(
                    "Paste after copy: waiting {}ms after paste shortcut...",
                    paste_delay_after_ms
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(paste_delay_after_ms)).await;
                crate::log_info!("Paste after copy: restoring clipboard...");
                typing::restore_clipboard(saved_clipboard);
                crate::log_info!("Paste after copy: complete");
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(paste_delay_after_ms)).await;
    Ok(())
}
