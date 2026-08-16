use crate::audio;
use crate::config::Config;
use crate::engine_factory;
use crate::platform;
use crate::post_process::factory::PostProcessFactory;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Lifecycle of a single dictation session. This is the authoritative guard
/// for hotkey gesture semantics and re-entrancy: a new session may only start
/// from `Idle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Recording,
    Transcribing,
    Typing,
}

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub session_state: Arc<Mutex<SessionState>>,
    /// Cancel token for the in-flight dictation session. Each session gets a
    /// fresh token so a cancelled pipeline can never clobber a newer session.
    pub active_session: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    pub is_mic_test_active: Arc<Mutex<bool>>,
    pub is_configuring_hotkey: Arc<Mutex<bool>>,
    pub hotkey_error: Arc<Mutex<Option<String>>>,
    pub hotkey_binding_state: Arc<Mutex<HotkeyBindingState>>,
    pub setup_status: Arc<Mutex<Option<String>>>,
    pub cached_device: Arc<Mutex<Option<cpal::Device>>>,
    pub playback_stream: Arc<Mutex<Option<cpal::Stream>>>,
    pub mic_test_samples: Arc<Mutex<Vec<f32>>>,
    pub audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    #[cfg(target_os = "linux")]
    pub hotkey_engine_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_sender:
        Arc<Mutex<Option<platform::linux::wayland::input::WaylandInputSender>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_ready: Arc<Mutex<bool>>,
    #[cfg(target_os = "linux")]
    pub wayland_host_app_registration_error: Arc<Mutex<Option<String>>>,
    pub display_backend: Arc<dyn platform::traits::DisplayBackend>,
    pub engine_factory: Arc<engine_factory::EngineFactory>,
    pub post_process_factory: Arc<PostProcessFactory>,
    pub python_runner: Arc<Mutex<Option<crate::python_runner::PythonRunner>>>,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct HotkeyBindingState {
    pub bound: bool,
    pub listening: bool,
    pub detail: Option<String>,
    pub active_trigger: Option<String>,
}

impl AppState {
    /// Terminate all child processes and release OS resources synchronously.
    /// Called before app exit so children (Python runner, llama-server, etc.)
    /// are killed and their ports freed before the process terminates.
    pub fn cleanup(&self) {
        crate::log_info!("AppState: cleaning up child processes");

        // 1. Kill Python runner (uvicorn) — extracting from Arc drops the
        //    Child handle, which triggers kill_on_drop(true) → SIGKILL.
        {
            let mut guard = self.python_runner.lock().unwrap();
            if guard.take().is_some() {
                crate::log_info!("Python runner terminated");
            }
        }

        // 2. Kill post-process sidecar (llama-server).
        self.post_process_factory.invalidate_local();

        // 3. Stop the persistent audio engine.
        {
            let mut guard = self.audio_engine.lock().unwrap();
            if guard.take().is_some() {
                crate::log_info!("Audio engine stopped");
            }
        }

        // 4. Stop any playback stream.
        {
            let mut guard = self.playback_stream.lock().unwrap();
            if guard.take().is_some() {
                crate::log_info!("Playback stream stopped");
            }
        }

        // 5. Cancel any in-flight dictation session.
        {
            let mut guard = self.active_session.lock().unwrap();
            *guard = None;
        }

        // 6. Cancel Wayland portal sessions (Linux).
        #[cfg(target_os = "linux")]
        {
            if let Some(sender) = self.hotkey_engine_cancel.lock().unwrap().take() {
                let _ = sender.send(());
            }
            if let Some(sender) = self.wayland_input_cancel.lock().unwrap().take() {
                let _ = sender.send(());
            }
            self.wayland_input_sender.lock().unwrap().take();
        }

        crate::log_info!("AppState: cleanup complete");
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(Mutex::new(Config::default())),
            session_state: Arc::new(Mutex::new(SessionState::Idle)),
            active_session: Arc::new(Mutex::new(None)),
            is_mic_test_active: Arc::new(Mutex::new(false)),
            is_configuring_hotkey: Arc::new(Mutex::new(false)),
            hotkey_error: Arc::new(Mutex::new(None)),
            hotkey_binding_state: Arc::new(Mutex::new(HotkeyBindingState::default())),
            setup_status: Arc::new(Mutex::new(None)),
            cached_device: Arc::new(Mutex::new(None)),
            playback_stream: Arc::new(Mutex::new(None)),
            mic_test_samples: Arc::new(Mutex::new(Vec::new())),
            audio_engine: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            hotkey_engine_cancel: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_sender: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_cancel: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_ready: Arc::new(Mutex::new(false)),
            #[cfg(target_os = "linux")]
            wayland_host_app_registration_error: Arc::new(Mutex::new(None)),
            display_backend: platform::initialize(),
            engine_factory: Arc::new(engine_factory::EngineFactory::new()),
            post_process_factory: Arc::new(PostProcessFactory::new()),
            python_runner: Arc::new(Mutex::new(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_defaults_to_idle() {
        assert_eq!(SessionState::Idle, SessionState::Idle);
    }

    #[test]
    fn session_state_transitions_are_distinct() {
        let states = [
            SessionState::Idle,
            SessionState::Recording,
            SessionState::Transcribing,
            SessionState::Typing,
        ];
        for i in 0..states.len() {
            for j in 0..states.len() {
                if i == j {
                    assert_eq!(states[i], states[j]);
                } else {
                    assert_ne!(states[i], states[j]);
                }
            }
        }
    }

    #[test]
    fn session_state_cycle() {
        let mut state = SessionState::Idle;
        assert_eq!(state, SessionState::Idle);

        state = SessionState::Recording;
        assert_eq!(state, SessionState::Recording);

        state = SessionState::Transcribing;
        assert_eq!(state, SessionState::Transcribing);

        state = SessionState::Typing;
        assert_eq!(state, SessionState::Typing);

        state = SessionState::Idle;
        assert_eq!(state, SessionState::Idle);
    }

    #[test]
    fn session_state_debug_output() {
        let s = format!("{:?}", SessionState::Idle);
        assert_eq!(s, "Idle");
        assert_eq!(format!("{:?}", SessionState::Recording), "Recording");
        assert_eq!(format!("{:?}", SessionState::Transcribing), "Transcribing");
        assert_eq!(format!("{:?}", SessionState::Typing), "Typing");
    }
}
