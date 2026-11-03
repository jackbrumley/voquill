//! Single owner of the persistent capture engine's lifecycle.
//!
//! Every path that needs the microphone (startup, dictation, mic test, voice
//! macros, permission requests, settings changes) goes through
//! `ensure_capture_engine`. It resolves the configured input device to the
//! endpoint it means *now* and (re)builds the engine whenever the engine is
//! missing, its stream has failed, or it is bound to a different endpoint
//! ("System Default" moved to another device, a device was re-plugged, the
//! selection changed). A configured device that cannot be resolved tears the
//! engine down, because the audio server would otherwise keep the stream alive
//! on a fallback microphone.
//!
//! The watchdog runs the same check while nothing is recording, so device
//! changes between dictations take effect without waiting for the next use.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::app::state::SessionState;
use crate::config::Config;
use crate::{audio, AppState};

/// How often the watchdog re-checks the capture endpoint while idle.
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(3);

/// Emitted whenever the capture engine starts, is replaced or is torn down,
/// so the UI re-probes devices and readiness immediately.
const AUDIO_DEVICES_CHANGED_EVENT: &str = "audio-devices-changed";

/// The config values the capture engine is built from.
pub struct CaptureSettings {
    pub device: Option<String>,
    pub sensitivity: f32,
}

impl CaptureSettings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            device: config.audio_device.clone(),
            sensitivity: config.input_sensitivity,
        }
    }

    fn current(app_handle: &AppHandle) -> Self {
        let state = app_handle.state::<AppState>();
        let config = state.config.lock().unwrap();
        Self::from_config(&config)
    }
}

/// Makes the capture engine run on the endpoint `settings` resolve to now.
pub fn ensure_capture_engine(
    app_handle: &AppHandle,
    settings: &CaptureSettings,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut engine_slot = state.audio_engine.lock().unwrap();

    let endpoint = match audio::resolve_capture_endpoint(settings.device.as_deref()) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            let error = format!(
                "Input device '{}' is unavailable: {}",
                settings.device.as_deref().unwrap_or("default"),
                error
            );
            if engine_slot.take().is_some() {
                drop(engine_slot);
                crate::log_warn!("Capture engine stopped: {}", error);
                announce_capture_change(app_handle);
            }
            return Err(error);
        }
    };

    let Some(reason) = rebuild_reason(engine_slot.as_ref(), &endpoint, settings) else {
        return Ok(());
    };
    crate::log_info!(
        "Starting capture engine on {} ({})",
        endpoint.identity,
        reason
    );

    match audio::PersistentAudioEngine::new(&endpoint, settings.sensitivity) {
        Ok(engine) => {
            *engine_slot = Some(engine);
            drop(engine_slot);
            announce_capture_change(app_handle);
            Ok(())
        }
        Err(error) => {
            let error = format!(
                "Failed to start capture on {}: {}",
                endpoint.identity, error
            );
            if engine_slot.take().is_some() {
                drop(engine_slot);
                crate::log_warn!("Capture engine stopped: {}", error);
                announce_capture_change(app_handle);
            }
            Err(error)
        }
    }
}

/// Starts the capture engine for the loaded config and the watchdog that
/// keeps it on the configured endpoint.
pub fn start(app_handle: &AppHandle) {
    if let Err(error) = ensure_capture_engine(app_handle, &CaptureSettings::current(app_handle)) {
        crate::log_warn!("Capture engine not started: {}", error);
    }
    spawn_watchdog(app_handle.clone());
}

fn rebuild_reason(
    engine: Option<&audio::PersistentAudioEngine>,
    endpoint: &audio::CaptureEndpoint,
    settings: &CaptureSettings,
) -> Option<String> {
    let Some(engine) = engine else {
        return Some("no engine running".to_string());
    };
    if let Some(failure) = engine.stream_failure() {
        return Some(format!("previous stream failed: {}", failure));
    }
    if engine.endpoint_identity != endpoint.identity {
        return Some(format!(
            "endpoint changed from {}",
            engine.endpoint_identity
        ));
    }
    if engine.sensitivity != settings.sensitivity {
        return Some(format!(
            "input sensitivity changed from {:.2} to {:.2}",
            engine.sensitivity, settings.sensitivity
        ));
    }
    None
}

/// Re-attaches the voice macro listener to the current engine and tells the
/// UI to re-probe devices and readiness.
fn announce_capture_change(app_handle: &AppHandle) {
    crate::voice_macro::sync_voice_macro_listener(app_handle);
    let _ = app_handle.emit(AUDIO_DEVICES_CHANGED_EVENT, ());
}

fn spawn_watchdog(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut reported_error: Option<String> = None;
        loop {
            tokio::time::sleep(WATCHDOG_INTERVAL).await;
            if !is_capture_idle(&app_handle) {
                continue;
            }

            let check_handle = app_handle.clone();
            let outcome = tauri::async_runtime::spawn_blocking(move || {
                ensure_capture_engine(&check_handle, &CaptureSettings::current(&check_handle))
            })
            .await;

            match outcome {
                Ok(Ok(())) => {
                    if reported_error.take().is_some() {
                        crate::log_info!("Capture watchdog: configured input device is back");
                    }
                }
                Ok(Err(error)) => {
                    if reported_error.as_deref() != Some(error.as_str()) {
                        crate::log_warn!("Capture watchdog: {}", error);
                        reported_error = Some(error);
                    }
                }
                Err(error) => crate::log_warn!("Capture watchdog check panicked: {}", error),
            }
        }
    });
}

/// Nothing is capturing: rebuilding the engine now cannot cut off a
/// dictation or a running mic test.
fn is_capture_idle(app_handle: &AppHandle) -> bool {
    let state = app_handle.state::<AppState>();
    let session_idle = *state.session_state.lock().unwrap() == SessionState::Idle;
    let mic_test_active = *state.is_mic_test_active.lock().unwrap();
    session_idle && !mic_test_active
}
