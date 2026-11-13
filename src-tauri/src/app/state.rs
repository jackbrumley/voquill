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
    /// Owned by `app::capture`; everything else only reads from it.
    pub audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    pub audio_player: Arc<audio::AudioPlayer>,
    #[cfg(target_os = "linux")]
    pub hotkey_engine_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_sender:
        Arc<Mutex<Option<platform::linux::wayland::input::WaylandInputSender>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_session: Arc<Mutex<platform::linux::wayland::input::InputSessionState>>,
    #[cfg(target_os = "linux")]
    pub wayland_host_app_registration_error: Arc<Mutex<Option<String>>>,
    pub display_backend: Arc<dyn platform::traits::DisplayBackend>,
    pub engine_factory: Arc<engine_factory::EngineFactory>,
    pub post_process_factory: Arc<PostProcessFactory>,
    pub python_runner: Arc<Mutex<Option<crate::python_runner::PythonRunner>>>,
    pub python_runner_init_lock: Arc<tokio::sync::Mutex<()>>,
    /// Why the most recent noise reduction attempt failed, or `None` after a
    /// success. Owned by `recording_flow::audio_processing::run_noise_reduction`.
    pub noise_reduction_error: Arc<Mutex<Option<String>>>,
    pub voice_macro_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
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
    /// Called only from the `RunEvent::Exit` handler in `main.rs`, which every
    /// exit path reaches, so children (Python runner, llama-server, etc.) are
    /// killed and their ports freed before the process terminates.
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

        // 4. Stop all playback.
        self.audio_player.stop_all();

        // 5. Cancel any in-flight dictation session.
        {
            let mut guard = self.active_session.lock().unwrap();
            *guard = None;
        }

        // 6. Cancel Voice Macro listener.
        {
            let mut guard = self.voice_macro_cancel.lock().unwrap();
            if let Some(cancel) = guard.take() {
                let _ = cancel.send(());
            }
        }

        // 7. Cancel Wayland portal sessions (Linux).
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

    /// Plays `clip` on the configured playback device. The single place that
    /// applies `playback_device`, so a new selection takes effect for the next
    /// sound everywhere.
    pub fn play_audio(
        &self,
        slot: audio::PlaybackSlot,
        clip: audio::AudioClip,
        on_end: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        let playback_device = self.config.lock().unwrap().playback_device.clone();
        self.audio_player.play(slot, clip, playback_device, on_end)
    }

    /// Retrieve the running Python runner instance, starting it if not already active.
    /// Thread-safe and serialized with an async lock to prevent duplicate sidecar spawns.
    pub async fn get_or_start_python_runner(
        &self,
        app_handle: &tauri::AppHandle,
    ) -> Result<crate::python_runner::PythonRunner, String> {
        {
            let guard = self.python_runner.lock().unwrap();
            if let Some(runner) = guard.as_ref() {
                return Ok(runner.clone());
            }
        }

        let _init_guard = self.python_runner_init_lock.lock().await;

        {
            let guard = self.python_runner.lock().unwrap();
            if let Some(runner) = guard.as_ref() {
                return Ok(runner.clone());
            }
        }

        let runner = crate::python_runner::PythonRunner::start(app_handle).await?;
        let mut guard = self.python_runner.lock().unwrap();
        *guard = Some(runner.clone());
        Ok(runner)
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
            audio_engine: Arc::new(Mutex::new(None)),
            audio_player: Arc::new(audio::AudioPlayer::default()),
            #[cfg(target_os = "linux")]
            hotkey_engine_cancel: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_sender: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_cancel: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_session: Arc::new(Mutex::new(
                platform::linux::wayland::input::InputSessionState::Inactive,
            )),
            #[cfg(target_os = "linux")]
            wayland_host_app_registration_error: Arc::new(Mutex::new(None)),
            display_backend: platform::initialize(),
            engine_factory: Arc::new(engine_factory::EngineFactory::new()),
            post_process_factory: Arc::new(PostProcessFactory::new()),
            python_runner: Arc::new(Mutex::new(None)),
            python_runner_init_lock: Arc::new(tokio::sync::Mutex::new(())),
            noise_reduction_error: Arc::new(Mutex::new(None)),
            voice_macro_cancel: Arc::new(Mutex::new(None)),
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
