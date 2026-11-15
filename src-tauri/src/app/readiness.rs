//! The single owner of "can Voquill be used right now": permissions, a real
//! microphone, a downloaded speech model (or an API key) and a registered
//! hotkey. The backend owns every one of these inputs, so it decides here and
//! the UI only displays the verdict.
//!
//! `evaluate` gathers the inputs and decides; `publish` also tells the
//! windows (`app-readiness-changed`) when the verdict changed. Callers that
//! change an input call `publish`; dictation evaluates fresh before it
//! starts; voice macros, which check on every audio frame, read the latest
//! cached verdict.

use crate::app::state::AppState;
use crate::audio::AudioDevice;
use crate::config::{Config, TranscriptionMode};
use crate::engines::ModelEngine;
use crate::model_manager::ModelManager;
use crate::platform::permissions::PlatformPermissions;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub const READINESS_CHANGED_EVENT: &str = "app-readiness-changed";
/// What an empty API key field is saved as.
pub const API_KEY_PLACEHOLDER: &str = "your_api_key_here";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioDeviceIssue {
    None,
    NoDevices,
    DeviceMissing,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessStatus {
    pub is_permissions_ready: bool,
    pub audio_device_issue: AudioDeviceIssue,
    pub is_audio_device_ready: bool,
    pub is_transcription_ready: bool,
    pub is_hotkey_ready: bool,
    pub is_all_ready: bool,
    /// Post-processing is on with a local model that is not downloaded.
    /// Advisory: it does not block dictation.
    pub post_process_model_missing: bool,
    /// Why the app is not ready, in plain words; empty when it is.
    pub reasons: Vec<String>,
}

/// Everything readiness depends on, gathered by `evaluate`.
pub struct ReadinessInputs<'a> {
    pub permissions: &'a PlatformPermissions,
    pub hotkey_error: Option<&'a str>,
    pub input_devices: &'a [AudioDevice],
    pub config: &'a Config,
    pub model_downloaded: &'a dyn Fn(ModelEngine, &str) -> bool,
}

/// The pure decision, unit-tested without any hardware.
pub fn decide(inputs: &ReadinessInputs) -> ReadinessStatus {
    let ReadinessInputs {
        permissions,
        hotkey_error,
        input_devices,
        config,
        model_downloaded,
    } = inputs;
    let mut reasons = Vec::new();

    if !permissions.audio {
        reasons.push("microphone access is not granted".to_string());
    }
    if !permissions.shortcuts {
        reasons.push("the global shortcut is not granted".to_string());
    }
    if permissions.input_emulation_restoring {
        reasons.push("the typing permission is still being restored".to_string());
    } else if !permissions.input_emulation {
        reasons.push("typing into other apps is not granted".to_string());
    }
    let is_permissions_ready =
        permissions.audio && permissions.shortcuts && permissions.input_emulation;

    let real_devices: Vec<&AudioDevice> = input_devices
        .iter()
        .filter(|device| !device.is_system_default)
        .collect();
    let configured_device = config.audio_device.as_deref().unwrap_or("default");
    let audio_device_issue = if real_devices.is_empty() {
        reasons.push("no microphone is connected".to_string());
        AudioDeviceIssue::NoDevices
    } else if configured_device != "default"
        && !real_devices
            .iter()
            .any(|device| device.id == configured_device)
    {
        reasons.push(format!(
            "the chosen microphone '{}' is not connected",
            configured_device
        ));
        AudioDeviceIssue::DeviceMissing
    } else {
        AudioDeviceIssue::None
    };

    let is_transcription_ready = match config.transcription_mode {
        TranscriptionMode::Local => {
            let ready = ModelEngine::parse(&config.local_engine)
                .is_some_and(|engine| model_downloaded(engine, &config.local_model_size));
            if !ready {
                reasons.push(format!(
                    "the speech model '{}' for '{}' is not downloaded",
                    config.local_model_size, config.local_engine
                ));
            }
            ready
        }
        TranscriptionMode::Api => {
            let key = config.openai_api_key.trim();
            let ready = !key.is_empty() && key != API_KEY_PLACEHOLDER;
            if !ready {
                reasons.push("no API key is set for cloud transcription".to_string());
            }
            ready
        }
    };

    if let Some(error) = hotkey_error {
        reasons.push(format!("the hotkey is not registered: {}", error));
    }
    let is_hotkey_ready = hotkey_error.is_none();

    let post_process_model_missing = config.post_process_enabled
        && config.post_process_provider == crate::config::post_process::PostProcessProvider::Local
        && !ModelEngine::parse(&config.post_process_engine)
            .is_some_and(|engine| model_downloaded(engine, &config.post_process_model));

    let is_audio_device_ready = audio_device_issue == AudioDeviceIssue::None;
    ReadinessStatus {
        is_permissions_ready,
        audio_device_issue,
        is_audio_device_ready,
        is_transcription_ready,
        is_hotkey_ready,
        is_all_ready: is_permissions_ready
            && is_audio_device_ready
            && is_transcription_ready
            && is_hotkey_ready,
        post_process_model_missing,
        reasons,
    }
}

/// The latest verdict, kept for checks too frequent to evaluate each time.
#[derive(Default)]
pub struct ReadinessCache {
    latest: Mutex<Option<ReadinessStatus>>,
}

impl ReadinessCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the latest verdict was ready; false before the first one.
    pub fn latest_is_ready(&self) -> bool {
        self.latest
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|status| status.is_all_ready)
    }

    /// Stores `status`, returning whether it differs from the previous one.
    fn store(&self, status: &ReadinessStatus) -> bool {
        let mut latest = self.latest.lock().unwrap();
        let changed = latest.as_ref() != Some(status);
        *latest = Some(status.clone());
        changed
    }
}

/// Gathers the inputs and decides, caching the verdict.
pub async fn evaluate(app_handle: &AppHandle) -> ReadinessStatus {
    evaluate_and_cache(app_handle).await.0
}

/// Evaluates, caches, and reports whether the verdict changed.
async fn evaluate_and_cache(app_handle: &AppHandle) -> (ReadinessStatus, bool) {
    let state = app_handle.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    let permissions = crate::app::commands::platform::platform_setup_status(&state).await;
    let hotkey_error = state.hotkey_error.lock().unwrap().clone();
    let input_devices = crate::audio::get_input_devices().unwrap_or_else(|error| {
        crate::log_warn!("Readiness: could not list microphones: {}", error);
        Vec::new()
    });
    let model_manager = ModelManager::new().ok();
    let model_downloaded = |engine: ModelEngine, size: &str| {
        let Some(manager) = model_manager.as_ref() else {
            return false;
        };
        ModelManager::find_model(engine, size)
            .is_some_and(|model| manager.is_model_downloaded(&model))
    };

    let status = decide(&ReadinessInputs {
        permissions: &permissions,
        hotkey_error: hotkey_error.as_deref(),
        input_devices: &input_devices,
        config: &config,
        model_downloaded: &model_downloaded,
    });
    let changed = state.readiness.store(&status);
    if changed {
        crate::log_info!(
            "App readiness: ready={}{}",
            status.is_all_ready,
            if status.reasons.is_empty() {
                String::new()
            } else {
                format!(" ({})", status.reasons.join("; "))
            }
        );
    }
    (status, changed)
}

/// Re-evaluates after an input changed and tells the windows if the verdict
/// changed.
pub async fn publish(app_handle: &AppHandle) {
    let (status, changed) = evaluate_and_cache(app_handle).await;
    if changed {
        let _ = app_handle.emit(READINESS_CHANGED_EVENT, status);
    }
}

/// `publish` from code that cannot await.
pub fn spawn_publish(app_handle: &AppHandle) {
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        publish(&app_handle).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn granted() -> PlatformPermissions {
        PlatformPermissions {
            audio: true,
            shortcuts: true,
            input_emulation: true,
            input_emulation_restoring: false,
            shortcuts_status: "bound".to_string(),
            shortcuts_detail: None,
            manual_overlay_offset_supported: true,
            overlay_positioning_detail: None,
        }
    }

    fn microphone(id: &str) -> AudioDevice {
        AudioDevice {
            id: id.to_string(),
            label: id.to_string(),
            is_system_default: false,
        }
    }

    fn local_config() -> Config {
        Config {
            transcription_mode: TranscriptionMode::Local,
            local_engine: "Parakeet".to_string(),
            local_model_size: "parakeet-unified-en-0.6b".to_string(),
            audio_device: Some("default".to_string()),
            ..Config::default()
        }
    }

    fn decide_with(config: &Config, devices: &[AudioDevice], downloaded: bool) -> ReadinessStatus {
        let permissions = granted();
        let model_downloaded = move |_: ModelEngine, _: &str| downloaded;
        decide(&ReadinessInputs {
            permissions: &permissions,
            hotkey_error: None,
            input_devices: devices,
            config,
            model_downloaded: &model_downloaded,
        })
    }

    #[test]
    fn ready_when_everything_is_in_place() {
        let status = decide_with(&local_config(), &[microphone("pulse:mic")], true);
        assert!(status.is_all_ready);
        assert!(status.reasons.is_empty());
    }

    #[test]
    fn missing_model_is_not_ready() {
        let status = decide_with(&local_config(), &[microphone("pulse:mic")], false);
        assert!(!status.is_transcription_ready);
        assert!(!status.is_all_ready);
    }

    #[test]
    fn no_real_microphone_is_not_ready() {
        let system_default = AudioDevice {
            id: "default".to_string(),
            label: "System Default".to_string(),
            is_system_default: true,
        };
        let status = decide_with(&local_config(), &[system_default], true);
        assert_eq!(status.audio_device_issue, AudioDeviceIssue::NoDevices);
    }

    #[test]
    fn unplugged_chosen_microphone_is_reported() {
        let mut config = local_config();
        config.audio_device = Some("pulse:usb".to_string());
        let status = decide_with(&config, &[microphone("pulse:mic")], true);
        assert_eq!(status.audio_device_issue, AudioDeviceIssue::DeviceMissing);
    }

    #[test]
    fn placeholder_api_key_is_not_ready() {
        let mut config = local_config();
        config.transcription_mode = TranscriptionMode::Api;
        config.openai_api_key = API_KEY_PLACEHOLDER.to_string();
        let status = decide_with(&config, &[microphone("pulse:mic")], true);
        assert!(!status.is_transcription_ready);
    }

    #[test]
    fn serializes_for_the_frontend() {
        let status = decide_with(&local_config(), &[microphone("pulse:mic")], true);
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["isAllReady"], true);
        assert_eq!(json["audioDeviceIssue"], "none");
    }
}
