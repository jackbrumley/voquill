use post_process::{PostProcessPrompt, PostProcessProvider};
use serde::{Deserialize, Serialize};
use voice_macro::VoiceMacroCommand;

pub mod post_process;
pub mod storage;
pub mod voice_macro;

pub const INPUT_SENSITIVITY_MIN: f32 = 0.1;
pub const INPUT_SENSITIVITY_MAX: f32 = 2.0;
pub const MAX_RECORDING_DURATION_MINUTES_MIN: u64 = 1;
pub const MAX_RECORDING_DURATION_MINUTES_MAX: u64 = 180;
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
    #[serde(default = "default_typing_speed")]
    pub typing_speed_interval: f64,
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

fn default_api_key() -> String {
    "your_api_key_here".to_string()
}
fn default_api_url() -> String {
    "https://api.openai.com/v1/audio/transcriptions".to_string()
}
fn default_api_model() -> String {
    "gpt-transcribe".to_string()
}
fn default_transcription_mode() -> TranscriptionMode {
    TranscriptionMode::Local
}
fn default_local_model_size() -> String {
    "base".to_string()
}
fn default_local_engine() -> String {
    "Whisper.cpp (GPU)".to_string()
}
fn default_hotkey() -> String {
    "ctrl+shift+space".to_string()
}
fn default_typing_speed() -> f64 {
    0.001
}
fn default_key_press_duration() -> u64 {
    2
}
fn default_paste_delay_before_ms() -> u64 {
    60
}
fn default_paste_delay_after_ms() -> u64 {
    60
}
fn default_pixels_from_bottom() -> i32 {
    50
}
fn default_audio_device() -> Option<String> {
    Some("default".to_string())
}
fn default_playback_device() -> Option<String> {
    Some("default".to_string())
}
fn default_enable_recording_logs() -> bool {
    false
}
fn default_input_sensitivity() -> f32 {
    1.0
}
fn default_output_method() -> OutputMethod {
    OutputMethod::Clipboard
}
fn default_close_to_tray() -> bool {
    true
}

fn default_paste_after_copy() -> bool {
    true
}
#[cfg(target_os = "macos")]
fn default_paste_shortcut() -> PasteShortcut {
    PasteShortcut::CommandV
}

#[cfg(not(target_os = "macos"))]
fn default_paste_shortcut() -> PasteShortcut {
    PasteShortcut::ShiftInsert
}
fn default_copy_on_typewriter() -> bool {
    false
}
fn default_language() -> String {
    "auto".to_string()
}
fn default_post_roll_ms() -> u64 {
    0
}
fn default_hotkey_mode() -> HotkeyMode {
    HotkeyMode::Toggle
}
fn default_dictionary() -> Vec<String> {
    vec!["Voquill".to_string()]
}
fn default_filler_word_removal_enabled() -> bool {
    true
}
fn default_history_limit() -> usize {
    500
}
fn default_noise_reduction_strength() -> f32 {
    0.7
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_max_recording_duration_minutes() -> u64 {
    10
}

fn default_diarization_cluster_threshold() -> f32 {
    0.7
}

fn default_voice_macro_trigger_word() -> String {
    String::new()
}

fn default_voice_macro_sound_feedback() -> bool {
    true
}

fn default_voice_macro_suppress_overlay() -> bool {
    true
}

fn default_voice_macro_activation_threshold() -> f32 {
    0.035
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: default_api_key(),
            api_url: default_api_url(),
            api_model: default_api_model(),
            transcription_mode: default_transcription_mode(),
            local_model_size: default_local_model_size(),
            local_engine: default_local_engine(),
            hotkey: default_hotkey(),
            typing_speed_interval: default_typing_speed(),
            key_press_duration_ms: default_key_press_duration(),
            paste_delay_before_ms: default_paste_delay_before_ms(),
            paste_delay_after_ms: default_paste_delay_after_ms(),
            pixels_from_bottom: default_pixels_from_bottom(),
            close_to_tray: default_close_to_tray(),
            audio_device: default_audio_device(),
            playback_device: default_playback_device(),
            enable_recording_logs: default_enable_recording_logs(),
            input_sensitivity: default_input_sensitivity(),
            output_method: default_output_method(),
            copy_on_typewriter: default_copy_on_typewriter(),
            language: default_language(),
            shortcuts_token: None,
            input_token: None,
            post_roll_ms: default_post_roll_ms(),
            hotkey_mode: default_hotkey_mode(),
            max_recording_duration_minutes: default_max_recording_duration_minutes(),
            engine_config: None,
            dictionary: default_dictionary(),
            post_process_enabled: false,
            post_process_provider: post_process::default_post_process_provider(),
            post_process_engine: post_process::default_post_process_engine(),
            post_process_model: post_process::default_post_process_model(),
            post_process_api_url: post_process::default_post_process_api_url(),
            post_process_api_key: String::new(),
            post_process_api_model: post_process::default_post_process_api_model(),
            post_process_prompt: post_process::default_post_process_prompt(),
            post_process_threads: post_process::default_post_process_threads(),
            post_process_prompts: post_process::default_post_process_prompts(),
            post_process_selected_prompt_id: None,
            post_process_user_prompt_template:
                post_process::default_post_process_user_prompt_template(),
            post_process_max_output_tokens: post_process::default_post_process_max_output_tokens(),
            filler_word_removal_enabled: default_filler_word_removal_enabled(),
            custom_filler_words: Vec::new(),
            noise_reduction_enabled: false,
            noise_reduction_strength: default_noise_reduction_strength(),
            append_trailing_space: false,
            auto_submit: false,
            paste_after_copy: default_paste_after_copy(),
            paste_shortcut: default_paste_shortcut(),
            history_limit: default_history_limit(),
            log_level: default_log_level(),
            diarization_enabled_files: false,
            diarization_enabled_recording: false,
            diarization_cluster_threshold: default_diarization_cluster_threshold(),
            voice_macros_enabled: false,
            voice_macro_trigger_word: default_voice_macro_trigger_word(),
            voice_macro_sound_feedback: default_voice_macro_sound_feedback(),
            voice_macro_suppress_overlay: default_voice_macro_suppress_overlay(),
            voice_macro_activation_threshold: default_voice_macro_activation_threshold(),
            voice_macros: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(config.local_engine, "Whisper.cpp (GPU)");
        assert_eq!(config.post_process_engine, "Post-Process (GPU)");
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
