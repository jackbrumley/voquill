//! Single owner of the persistent capture engine's lifecycle: whether the
//! microphone is open, and on which endpoint.
//!
//! **When it is open.** The engine runs while something needs it:
//! - a standing need from the settings (`keeps_microphone_open`): the user
//!   keeps the microphone open between dictations, or voice macros are on and
//!   must keep listening;
//! - an active hold (`hold`): a dictation, a mic test, the spoken macro tester
//!   or a permission request. A `CaptureHold` lasts as long as its user and lets
//!   go when dropped, however that user ended (finished, cancelled, failed).
//!
//! With neither, the engine is torn down so the operating system sees the
//! microphone as unused (webcam lights turn off, headset bases switch to a
//! phone). Every open and close goes through `reconcile`, under the holds lock,
//! so a hold taken while the last one is being dropped can never lose its
//! engine.
//!
//! **Which endpoint.** `ensure_engine` resolves the configured input device to
//! the endpoint it means *now* and (re)builds the engine whenever it is
//! missing, its stream has failed, or it is bound to a different endpoint
//! ("System Default" moved to another device, a device was re-plugged, the
//! selection changed). A configured device that cannot be resolved tears the
//! engine down, because the audio server would otherwise keep the stream alive
//! on a fallback microphone.
//!
//! The watchdog runs the same reconciliation every few seconds while nothing
//! is recording, so device changes between dictations take effect without
//! waiting for the next use.

use std::sync::MutexGuard;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::app::state::SessionState;
use crate::config::{Config, MicrophoneWhenIdle};
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
}

/// What a `CaptureHold` holds the microphone open for; named in the log.
#[derive(Debug, Clone, Copy)]
pub enum CapturePurpose {
    Dictation,
    MicTest,
    SpokenMacroTest,
    /// macOS asks for microphone permission when a stream opens.
    #[cfg(target_os = "macos")]
    PermissionRequest,
}

/// Keeps the microphone open while it lives. Dropping it lets go; the engine
/// closes if nothing else needs it.
#[must_use = "the microphone is released as soon as the hold is dropped"]
pub struct CaptureHold {
    app_handle: AppHandle,
    purpose: CapturePurpose,
}

impl Drop for CaptureHold {
    fn drop(&mut self) {
        let state = self.app_handle.state::<AppState>();
        let mut holds = state.capture_holds.lock().unwrap();
        *holds = holds.saturating_sub(1);
        crate::log_info!(
            "Microphone hold released ({:?}, {} remaining)",
            self.purpose,
            *holds
        );
        let config = state.config.lock().unwrap().clone();
        if let Err(error) = reconcile(&self.app_handle, &holds, &config) {
            crate::log_warn!("Capture engine not kept: {}", error);
        }
    }
}

/// Whether the settings alone keep the microphone open while nothing is
/// recording. Voice macros listen all the time, so they keep it open whatever
/// `microphone_when_idle` says.
pub fn keeps_microphone_open(config: &Config) -> bool {
    config.microphone_when_idle == MicrophoneWhenIdle::KeepOpen || config.voice_macros_enabled
}

/// Opens the microphone (on the configured endpoint) for `purpose` and keeps
/// it open until the returned hold is dropped.
pub fn hold(app_handle: &AppHandle, purpose: CapturePurpose) -> Result<CaptureHold, String> {
    let state = app_handle.state::<AppState>();
    let mut holds = state.capture_holds.lock().unwrap();
    let settings = CaptureSettings::from_config(&state.config.lock().unwrap());
    ensure_engine(app_handle, &settings)?;
    *holds += 1;
    crate::log_info!("Microphone held ({:?}, {} active)", purpose, *holds);
    Ok(CaptureHold {
        app_handle: app_handle.clone(),
        purpose,
    })
}

/// Checks a new input selection before it is saved. When the microphone
/// should be open under `next_config` the engine moves to the new endpoint
/// (an unusable device fails here); otherwise the device only has to resolve,
/// so a selection can be checked without opening the microphone.
pub fn apply_input_change(app_handle: &AppHandle, next_config: &Config) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let holds = state.capture_holds.lock().unwrap();
    let settings = CaptureSettings::from_config(next_config);
    if *holds > 0 || keeps_microphone_open(next_config) {
        return ensure_engine(app_handle, &settings);
    }
    drop(holds);
    resolve_endpoint(&settings).map(|_| ())
}

/// Opens or closes the microphone to match the stored settings and active
/// holds. Called at startup, after every settings save and factory reset, and
/// by the watchdog.
pub fn sync_with_settings(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let holds = state.capture_holds.lock().unwrap();
    let config = state.config.lock().unwrap().clone();
    if let Err(error) = reconcile(app_handle, &holds, &config) {
        crate::log_warn!("Capture engine not started: {}", error);
    }
}

/// The one place that decides whether the engine runs. Takes the holds guard
/// so no hold can be taken or dropped while it decides.
fn reconcile(
    app_handle: &AppHandle,
    holds: &MutexGuard<'_, usize>,
    config: &Config,
) -> Result<(), String> {
    if **holds > 0 || keeps_microphone_open(config) {
        return ensure_engine(app_handle, &CaptureSettings::from_config(config));
    }
    release_engine(app_handle);
    Ok(())
}

/// Tears the engine down so the operating system sees the microphone as
/// unused.
fn release_engine(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let mut engine_slot = state.audio_engine.lock().unwrap();
    if engine_slot.take().is_some() {
        drop(engine_slot);
        crate::log_info!("Capture engine closed: nothing needs the microphone");
        announce_capture_change(app_handle);
    }
}

fn resolve_endpoint(settings: &CaptureSettings) -> Result<audio::CaptureEndpoint, String> {
    audio::resolve_capture_endpoint(settings.device.as_deref()).map_err(|error| {
        format!(
            "Input device '{}' is unavailable: {}",
            settings.device.as_deref().unwrap_or("default"),
            error
        )
    })
}

/// Makes the capture engine run on the endpoint `settings` resolve to now.
/// Only `reconcile`, `hold` and `apply_input_change` call it, each under the
/// holds lock.
fn ensure_engine(app_handle: &AppHandle, settings: &CaptureSettings) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut engine_slot = state.audio_engine.lock().unwrap();

    let endpoint = match resolve_endpoint(settings) {
        Ok(endpoint) => endpoint,
        Err(error) => {
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

/// Opens the microphone if the loaded settings keep it open, and starts the
/// watchdog that keeps the engine on the configured endpoint.
pub fn start(app_handle: &AppHandle) {
    sync_with_settings(app_handle);
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
    crate::app::readiness::spawn_publish(app_handle);
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
                let state = check_handle.state::<AppState>();
                let holds = state.capture_holds.lock().unwrap();
                let config = state.config.lock().unwrap().clone();
                reconcile(&check_handle, &holds, &config)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(microphone_when_idle: MicrophoneWhenIdle, voice_macros_enabled: bool) -> Config {
        Config {
            microphone_when_idle,
            voice_macros_enabled,
            ..Config::default()
        }
    }

    #[test]
    fn keep_open_holds_the_microphone_while_idle() {
        assert!(keeps_microphone_open(&config_with(
            MicrophoneWhenIdle::KeepOpen,
            false
        )));
    }

    #[test]
    fn release_lets_the_microphone_go_while_idle() {
        assert!(!keeps_microphone_open(&config_with(
            MicrophoneWhenIdle::Release,
            false
        )));
    }

    #[test]
    fn voice_macros_keep_the_microphone_open_even_when_set_to_release() {
        assert!(keeps_microphone_open(&config_with(
            MicrophoneWhenIdle::Release,
            true
        )));
    }
}
