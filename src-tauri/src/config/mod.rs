use post_process::{PostProcessPrompt, PostProcessProvider};
use serde::{Deserialize, Serialize};
use voice_macro::VoiceMacroCommand;

use defaults::*;

pub(crate) mod defaults;
pub mod post_process;
pub mod storage;
pub mod voice_macro;

pub const INPUT_SENSITIVITY_MIN: f32 = 0.1;
pub const INPUT_SENSITIVITY_MAX: f32 = 4.0;
pub const MAX_RECORDING_DURATION_MINUTES_MIN: u64 = 1;
pub const MAX_RECORDING_DURATION_MINUTES_MAX: u64 = 180;
/// Longest allowed idle time before the speech engine unloads (one day).
/// Zero means the engine never unloads by itself.
pub const ENGINE_IDLE_UNLOAD_MINUTES_MAX: u64 = 1440;
pub const DIARIZATION_CLUSTER_THRESHOLD_MIN: f32 = 0.3;
pub const DIARIZATION_CLUSTER_THRESHOLD_MAX: f32 = 0.95;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputMethod {
    Typewriter,
    Clipboard,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PasteShortcut {
    ShiftInsert,
    CtrlV,
    CtrlShiftV,
    CommandV,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranscriptionMode {
    #[serde(rename = "API")]
    Api,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HotkeyMode {
    HoldToTalk,
    Toggle,
}

/// What happens to other applications' audio while a dictation records.
/// Lowering it is the default: dictation is clearer and music returns as
/// soon as the recording stops. Where it is unsupported (macOS) it does
/// nothing.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum OtherAudioWhileRecording {
    Keep,
    #[default]
    Lower,
    Mute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_api_key")]
    pub openai_api_key: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default = "default_api_model")]
    pub api_model: String,
    #[serde(default = "default_transcription_mode")]
    pub transcription_mode: TranscriptionMode,
    #[serde(default = "default_local_model_size")]
    pub local_model_size: String,
    #[serde(default = "default_local_engine")]
    pub local_engine: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// Delay between typed characters in Typewriter mode, in milliseconds.
    #[serde(default = "default_typing_interval_ms")]
    pub typing_interval_ms: u64,
    #[serde(default = "default_key_press_duration")]
    pub key_press_duration_ms: u64,
    #[serde(default = "default_paste_delay_before_ms")]
    pub paste_delay_before_ms: u64,
    #[serde(default = "default_paste_delay_after_ms")]
    pub paste_delay_after_ms: u64,
    #[serde(default = "default_pixels_from_bottom")]
    pub pixels_from_bottom: i32,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
    #[serde(default = "default_audio_device")]
    pub audio_device: Option<String>,
    #[serde(default = "default_playback_device")]
    pub playback_device: Option<String>,
    #[serde(default = "default_enable_recording_logs")]
    pub enable_recording_logs: bool,
    #[serde(default = "default_input_sensitivity")]
    pub input_sensitivity: f32,
    #[serde(default = "default_output_method")]
    pub output_method: OutputMethod,
    #[serde(default = "default_copy_on_typewriter")]
    pub copy_on_typewriter: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub shortcuts_token: Option<String>,
    #[serde(default)]
    pub input_token: Option<String>,
    #[serde(default = "default_post_roll_ms")]
    pub post_roll_ms: u64,
    #[serde(default = "default_hotkey_mode")]
    pub hotkey_mode: HotkeyMode,
    #[serde(default = "default_max_recording_duration_minutes")]
    pub max_recording_duration_minutes: u64,
    /// The setup revision this user last finished or skipped (see
    /// `setup_review`). Owned by the backend: `save_config` never takes the
    /// frontend's value.
    #[serde(default)]
    pub completed_setup_revision: u32,
    /// Minutes without dictation after which the local speech engine is
    /// unloaded to free memory. Zero keeps it loaded.
    #[serde(default = "default_engine_idle_unload_minutes")]
    pub engine_idle_unload_minutes: u64,
    #[serde(default)]
    pub engine_config: Option<serde_json::Value>,
    #[serde(default = "default_dictionary")]
    pub dictionary: Vec<String>,
    #[serde(default)]
    pub post_process_enabled: bool,
    #[serde(default = "post_process::default_post_process_provider")]
    pub post_process_provider: PostProcessProvider,
    #[serde(default = "post_process::default_post_process_engine")]
    pub post_process_engine: String,
    #[serde(default = "post_process::default_post_process_model")]
    pub post_process_model: String,
    #[serde(default = "post_process::default_post_process_api_url")]
    pub post_process_api_url: String,
    #[serde(default)]
    pub post_process_api_key: String,
    #[serde(default = "post_process::default_post_process_api_model")]
    pub post_process_api_model: String,
    #[serde(default = "post_process::default_post_process_prompt")]
    pub post_process_prompt: String,
    #[serde(default = "post_process::default_post_process_threads")]
    pub post_process_threads: String,
    #[serde(default)]
    pub post_process_prompts: Vec<PostProcessPrompt>,
    #[serde(default)]
    pub post_process_selected_prompt_id: Option<String>,
    #[serde(default = "post_process::default_post_process_user_prompt_template")]
    pub post_process_user_prompt_template: String,
    #[serde(default = "post_process::default_post_process_max_output_tokens")]
    pub post_process_max_output_tokens: u32,
    #[serde(default = "default_filler_word_removal_enabled")]
    pub filler_word_removal_enabled: bool,
    #[serde(default)]
    pub custom_filler_words: Vec<String>,
    #[serde(default)]
    pub noise_reduction_enabled: bool,
    #[serde(default = "default_noise_reduction_strength")]
    pub noise_reduction_strength: f32,
    #[serde(default)]
    pub other_audio_while_recording: OtherAudioWhileRecording,
    /// When true, the capture stream is opened only while dictating or running
    /// a mic test, then released so shared headsets can switch to a phone.
    /// Ignored while voice macros are enabled (they need continuous listening).
    #[serde(default)]
    pub release_mic_when_idle: bool,
    #[serde(default = "default_append_trailing_space")]
    pub append_trailing_space: bool,
    #[serde(default)]
    pub auto_submit: bool,
    #[serde(default = "default_paste_after_copy")]
    pub paste_after_copy: bool,
    #[serde(default = "default_paste_shortcut")]
    pub paste_shortcut: PasteShortcut,
    #[serde(default = "default_history_limit")]
    pub history_limit: usize,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default, alias = "diarization_enabled")]
    pub diarization_enabled_files: bool,
    #[serde(default)]
    pub diarization_enabled_recording: bool,
    #[serde(default = "default_diarization_cluster_threshold")]
    pub diarization_cluster_threshold: f32,
    #[serde(default)]
    pub voice_macros_enabled: bool,
    #[serde(default = "default_voice_macro_trigger_word")]
    pub voice_macro_trigger_word: String,
    #[serde(default = "default_voice_macro_sound_feedback")]
    pub voice_macro_sound_feedback: bool,
    #[serde(default = "default_voice_macro_suppress_overlay")]
    pub voice_macro_suppress_overlay: bool,
    #[serde(default = "default_voice_macro_activation_threshold")]
    pub voice_macro_activation_threshold: f32,
    #[serde(default)]
    pub voice_macros: Vec<VoiceMacroCommand>,
}

impl Config {
    /// Builds a cleanly framed prompt hint for transcription models.
    /// Formats spelling conventions and custom dictionary terms as independent,
    /// period-terminated context tokens to prevent open-clause completion loops.
    pub fn resolve_prompt_hint(&self) -> Option<String> {
        let spelling_hint = match self.language.as_str() {
            "en-AU" => Some("Australian spelling."),
            "en-GB" => Some("British spelling."),
            "en-US" => Some("American spelling."),
            _ => None,
        };

        let mut parts = Vec::new();
        if let Some(hint) = spelling_hint {
            parts.push(hint.to_string());
        }
        if !self.dictionary.is_empty() {
            for word in &self.dictionary {
                let trimmed = word.trim();
                if !trimmed.is_empty() {
                    let mut term = trimmed.to_string();
                    if !term.ends_with('.') && !term.ends_with('!') && !term.ends_with('?') {
                        term.push('.');
                    }
                    parts.push(term);
                }
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }

    /// The configured transcription engine. Settings store its name; an
    /// unknown name is an error here rather than an unreadable settings file.
    pub fn transcription_engine(&self) -> Result<crate::engines::ModelEngine, String> {
        crate::engines::ModelEngine::parse_required(&self.local_engine)
    }

    /// The configured local post-processing engine.
    pub fn post_process_model_engine(&self) -> Result<crate::engines::ModelEngine, String> {
        crate::engines::ModelEngine::parse_required(&self.post_process_engine)
    }

    pub fn normalize(&mut self) {
        self.normalize_input_sensitivity();
        self.diarization_cluster_threshold = self.diarization_cluster_threshold.clamp(
            DIARIZATION_CLUSTER_THRESHOLD_MIN,
            DIARIZATION_CLUSTER_THRESHOLD_MAX,
        );
        self.max_recording_duration_minutes = self.max_recording_duration_minutes.clamp(
            MAX_RECORDING_DURATION_MINUTES_MIN,
            MAX_RECORDING_DURATION_MINUTES_MAX,
        );
        self.engine_idle_unload_minutes = self
            .engine_idle_unload_minutes
            .min(ENGINE_IDLE_UNLOAD_MINUTES_MAX);
        // Ensure built-in prompts exist (migration for users upgrading)
        let pirate_id = "pirate";
        if !self.post_process_prompts.iter().any(|p| p.id == pirate_id) {
            let defaults = post_process::default_post_process_prompts();
            if let Some(pirate) = defaults.into_iter().find(|p| p.id == pirate_id) {
                self.post_process_prompts.push(pirate);
            }
        }

        for cmd in &mut self.voice_macros {
            if cmd.steps.is_empty() {
                cmd.steps = cmd.resolve_steps();
            }
        }
    }

    fn normalize_input_sensitivity(&mut self) {
        self.input_sensitivity = self
            .input_sensitivity
            .clamp(INPUT_SENSITIVITY_MIN, INPUT_SENSITIVITY_MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_transcription_model_exists_in_the_catalog() {
        let config = Config::default();
        let engine = config.transcription_engine().unwrap();
        assert!(
            crate::model_manager::ModelManager::find_model(engine, &config.local_model_size)
                .is_some()
        );
    }

    #[test]
    fn resolve_prompt_hint_empty_auto_is_none() {
        let config = Config {
            language: "auto".to_string(),
            dictionary: vec![],
            ..Default::default()
        };
        assert_eq!(config.resolve_prompt_hint(), None);
    }

    #[test]
    fn resolve_prompt_hint_dictionary_only_has_terminal_period() {
        let config = Config {
            language: "auto".to_string(),
            dictionary: vec!["xylophone".to_string(), "Voquill".to_string()],
            ..Default::default()
        };
        assert_eq!(
            config.resolve_prompt_hint(),
            Some("xylophone. Voquill.".to_string())
        );
    }

    #[test]
    fn resolve_prompt_hint_with_spelling_and_dictionary() {
        let config = Config {
            language: "en-US".to_string(),
            dictionary: vec!["Voquill".to_string(), "llama".to_string()],
            ..Default::default()
        };
        assert_eq!(
            config.resolve_prompt_hint(),
            Some("American spelling. Voquill. llama.".to_string())
        );
    }

    #[test]
    fn resolve_prompt_hint_preserves_existing_terminal_punctuation() {
        let config = Config {
            language: "auto".to_string(),
            dictionary: vec!["Voquill!".to_string()],
            ..Default::default()
        };
        assert_eq!(config.resolve_prompt_hint(), Some("Voquill!".to_string()));
    }

    #[test]
    fn resolve_prompt_hint_spelling_only() {
        let config = Config {
            language: "en-GB".to_string(),
            dictionary: vec![],
            ..Default::default()
        };
        assert_eq!(
            config.resolve_prompt_hint(),
            Some("British spelling.".to_string())
        );
    }

    #[test]
    fn paste_shortcut_defaults_for_platform() {
        let config: Config = serde_json::from_str("{}").expect("deserialization should succeed");
        #[cfg(target_os = "macos")]
        assert_eq!(config.paste_shortcut, PasteShortcut::CommandV);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(config.paste_shortcut, PasteShortcut::ShiftInsert);
    }

    #[test]
    fn paste_shortcut_deserializes_variants() {
        let json = r#"{"paste_shortcut": "CtrlV"}"#;
        let config: Config = serde_json::from_str(json).expect("deserialization should succeed");
        assert_eq!(config.paste_shortcut, PasteShortcut::CtrlV);

        let json_shift = r#"{"paste_shortcut": "CtrlShiftV"}"#;
        let config_shift: Config =
            serde_json::from_str(json_shift).expect("deserialization should succeed");
        assert_eq!(config_shift.paste_shortcut, PasteShortcut::CtrlShiftV);

        let json_command = r#"{"paste_shortcut": "CommandV"}"#;
        let config_command: Config =
            serde_json::from_str(json_command).expect("deserialization should succeed");
        assert_eq!(config_command.paste_shortcut, PasteShortcut::CommandV);
    }

    #[test]
    fn default_config_matches_expected_out_of_the_box_values() {
        let config = Config::default();
        assert_eq!(config.local_engine, "Parakeet");
        assert_eq!(config.post_process_engine, "Post-Process (Local)");
        assert!(config.append_trailing_space);
        assert_eq!(config.post_roll_ms, 150);
        assert_eq!(config.max_recording_duration_minutes, 30);
        assert!(config.enable_recording_logs);
        assert_eq!(
            config.other_audio_while_recording,
            OtherAudioWhileRecording::Lower
        );
        assert_eq!(config.output_method, OutputMethod::Clipboard);
        assert!(config.paste_after_copy);
        #[cfg(target_os = "macos")]
        assert_eq!(config.paste_shortcut, PasteShortcut::CommandV);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(config.paste_shortcut, PasteShortcut::ShiftInsert);
        assert_eq!(config.hotkey_mode, HotkeyMode::Toggle);
        assert_eq!(config.pixels_from_bottom, 50);
        assert!(config.close_to_tray);
        assert_eq!(config.dictionary, vec!["Voquill".to_string()]);
        assert_eq!(config.post_process_threads, "auto");
    }
}
