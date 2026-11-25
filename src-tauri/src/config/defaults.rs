//! Every default setting value, and `Config::default()` built from them.
//! The serde `default = "..."` attributes on `Config` name these functions.

use super::post_process;
use super::{
    Config, HotkeyMode, OtherAudioWhileRecording, OutputMethod, PasteShortcut, TranscriptionMode,
};

pub(super) fn default_api_key() -> String {
    "your_api_key_here".to_string()
}
pub(super) fn default_api_url() -> String {
    "https://api.openai.com/v1/audio/transcriptions".to_string()
}
pub(super) fn default_api_model() -> String {
    "gpt-transcribe".to_string()
}
pub(super) fn default_transcription_mode() -> TranscriptionMode {
    TranscriptionMode::Local
}
/// Defaults come from the setup recommendation rules (the single owner of
/// which engine and model suit a computer), using their hardware-independent
/// choice.
pub(super) fn default_local_model_size() -> String {
    crate::setup_recommendation::baseline_choice().1.to_string()
}
pub(super) fn default_local_engine() -> String {
    crate::setup_recommendation::baseline_choice()
        .0
        .name()
        .to_string()
}
pub(super) fn default_hotkey() -> String {
    "ctrl+shift+space".to_string()
}
pub(super) fn default_typing_interval_ms() -> u64 {
    1
}
pub(super) fn default_key_press_duration() -> u64 {
    2
}
pub(super) fn default_paste_delay_before_ms() -> u64 {
    60
}
pub(super) fn default_paste_delay_after_ms() -> u64 {
    60
}
pub(super) fn default_pixels_from_bottom() -> i32 {
    50
}
pub(super) fn default_audio_device() -> Option<String> {
    Some("default".to_string())
}
pub(super) fn default_playback_device() -> Option<String> {
    Some("default".to_string())
}
/// Keeps each dictation's audio so History can play it back. Old
/// recordings are deleted along with old History entries.
pub(super) fn default_enable_recording_logs() -> bool {
    true
}
/// Consecutive dictations would otherwise run into each other.
pub(super) fn default_append_trailing_space() -> bool {
    true
}
pub(super) fn default_input_sensitivity() -> f32 {
    1.0
}
pub(super) fn default_output_method() -> OutputMethod {
    OutputMethod::Clipboard
}
pub(super) fn default_close_to_tray() -> bool {
    true
}

pub(super) fn default_paste_after_copy() -> bool {
    true
}
#[cfg(target_os = "macos")]
pub(super) fn default_paste_shortcut() -> PasteShortcut {
    PasteShortcut::CommandV
}

#[cfg(not(target_os = "macos"))]
pub(super) fn default_paste_shortcut() -> PasteShortcut {
    PasteShortcut::ShiftInsert
}
pub(super) fn default_copy_on_typewriter() -> bool {
    false
}
pub(super) fn default_language() -> String {
    "auto".to_string()
}
/// Catches the end of the last word when the hotkey is pressed slightly early.
pub(super) fn default_post_roll_ms() -> u64 {
    150
}
pub(super) fn default_hotkey_mode() -> HotkeyMode {
    HotkeyMode::Toggle
}
pub(super) fn default_dictionary() -> Vec<String> {
    vec!["Voquill".to_string()]
}
pub(super) fn default_filler_word_removal_enabled() -> bool {
    true
}
pub(super) fn default_history_limit() -> usize {
    500
}
pub(super) fn default_noise_reduction_strength() -> f32 {
    0.7
}
pub(super) fn default_log_level() -> String {
    "info".to_string()
}
/// Frees the speech engine's memory after half an hour without dictation.
pub const DEFAULT_ENGINE_IDLE_UNLOAD_MINUTES: u64 = 30;

pub(super) fn default_engine_idle_unload_minutes() -> u64 {
    DEFAULT_ENGINE_IDLE_UNLOAD_MINUTES
}
pub(super) fn default_max_recording_duration_minutes() -> u64 {
    30
}

pub(super) fn default_diarization_cluster_threshold() -> f32 {
    0.7
}

pub(super) fn default_voice_macro_trigger_word() -> String {
    String::new()
}

pub(super) fn default_voice_macro_sound_feedback() -> bool {
    true
}

pub(super) fn default_voice_macro_suppress_overlay() -> bool {
    true
}

pub(super) fn default_voice_macro_activation_threshold() -> f32 {
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
            typing_interval_ms: default_typing_interval_ms(),
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
            engine_idle_unload_minutes: default_engine_idle_unload_minutes(),
            completed_setup_revision: 0,
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
            other_audio_while_recording: OtherAudioWhileRecording::default(),
            release_mic_when_idle: false,
            append_trailing_space: default_append_trailing_space(),
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
