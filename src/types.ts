export interface PostProcessPrompt {
  id: string;
  name: string;
  prompt: string;
  user_prompt_template?: string | null;
  max_output_tokens?: number | null;
}

export type MacroStep =
  | { type: 'KeyPress'; key: string; hold_ms?: number }
  | { type: 'KeyDown'; key: string }
  | { type: 'KeyUp'; key: string }
  | { type: 'Delay'; duration_ms: number }
  | { type: 'TypeText'; text: string }
  | { type: 'RunCommand'; command: string };

export type MacroSoundMode = 'default' | 'none' | 'tts' | 'custom_file' | 'mic_recording';

export interface VoicePersonaInfo {
  id: string;
  name: string;
  persona: string;
  category: string;
  engine: string;
  description: string;
  is_ready: boolean;
}

export interface VoiceMacroCommand {
  id: string;
  phrase: string;
  phrases?: string[];
  steps: MacroStep[];
  key_combination?: string | null;
  hold_ms?: number | null;
  delay_after_ms?: number | null;
  sound_mode?: MacroSoundMode;
  sound_tts_text?: string | null;
  sound_tts_voice?: string | null;
  sound_tts_speed?: number | null;
  sound_tts_effect?: string | null;
  sound_tts_pitch?: number | null;
}

export type PasteShortcut = 'ShiftInsert' | 'CtrlV' | 'CtrlShiftV';

export interface Config {
  openai_api_key: string;
  api_url: string;
  api_model: string;
  transcription_mode: 'API' | 'Local';
  local_model_size: string;
  local_engine: string;
  hotkey: string;
  typing_speed_interval: number;
  key_press_duration_ms: number;
  pixels_from_bottom: number;
  audio_device: string | null;
  playback_device?: string | null;
  enable_recording_logs: boolean;
  input_sensitivity: number;
  output_method: 'Typewriter' | 'Clipboard';
  copy_on_typewriter: boolean;
  language: string;
  post_roll_ms: number;
  hotkey_mode: 'HoldToTalk' | 'Toggle';
  max_recording_duration_minutes: number;
  engine_config: Record<string, unknown> | null;
  dictionary: string[];
  post_process_enabled: boolean;
  post_process_provider: 'Local' | 'API';
  post_process_engine: string;
  post_process_model: string;
  post_process_api_url: string;
  post_process_api_key: string;
  post_process_api_model: string;
  post_process_prompt: string;
  post_process_threads: string;
  post_process_user_prompt_template: string;
  post_process_max_output_tokens: number;
  post_process_prompts: PostProcessPrompt[];
  post_process_selected_prompt_id: string | null;
  filler_word_removal_enabled: boolean;
  custom_filler_words: string[];
  append_trailing_space: boolean;
  auto_submit: boolean;
  paste_after_copy: boolean;
  paste_shortcut: PasteShortcut;
  noise_reduction_enabled: boolean;
  noise_reduction_strength: number;
  history_limit: number;
  log_level: string;
  diarization_enabled_files: boolean;
  diarization_enabled_recording: boolean;
  diarization_cluster_threshold: number;
  voice_macros_enabled: boolean;
  voice_macro_trigger_word: string;
  voice_macro_sound_feedback: boolean;
  voice_macro_suppress_overlay: boolean;
  voice_macro_activation_threshold: number;
  voice_macros: VoiceMacroCommand[];
  shortcuts_token?: string;
  input_token?: string;
}

export interface Toast {
  id: number;
  message: string;
  type: 'success' | 'error' | 'info' | 'saved';
}

export interface HistoryItem {
  id: number;
  session_uuid?: string;
  text: string;
  timestamp: string;
  status?: 'success' | 'empty' | 'failed' | 'cancelled' | string;
  raw_text?: string | null;
  error_message?: string | null;
  segments?: Segment[] | null;
  audio_file?: string | null;
  duration_secs?: number | null;
  engine?: string | null;
  source?: 'mic' | 'file' | string | null;
  language?: string | null;
  prompt_name?: string | null;
}

export interface Segment {
  speaker: string | null;
  text: string;
  start_sec: number | null;
  end_sec: number | null;
}

export interface AudioDevice {
  id: string;
  label: string;
  is_system_default: boolean;
}

export interface MicVolumePayload {
  volume: number;
  is_triggered: boolean;
}

export interface LinuxPermissions {
  audio: boolean;
  shortcuts: boolean;
  input_emulation: boolean;
  shortcuts_status: string;
  shortcuts_detail?: string;
  manual_overlay_offset_supported?: boolean;
  overlay_positioning_detail?: string;
}

export interface ConfigureHotkeyResult {
  outcome: 'configured' | 'requires_in_app_capture' | 'system_managed';
  detail?: string;
}

export interface HotkeyBindingState {
  bound: boolean;
  listening: boolean;
  detail?: string;
  active_trigger?: string;
}

export interface SystemShortcutContext {
  distro?: string;
  desktop?: string;
  settings_path: string;
}

export interface OverlayPositioningCapabilities {
  manual_offset_supported: boolean;
  detail?: string;
}

export interface ModelInfo {
  engine: string;
  size: string;
  file_size: number;
  download_url: string;
  sha256: string;
  label: string;
  description: string;
  recommended: boolean;
  category: string;
}

export type DownloadPhase = 'downloading' | 'extracting';

export interface ModelDownloadProgress {
  phase: DownloadPhase;
  progress: number;
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  releaseUrl: string;
  notesUrl?: string;
}

export interface GpuStatus {
  tested: boolean;
  available: boolean;
  detail: string | null;
}

export interface SettingOption {
  value: string;
  label: string;
}

export interface EngineSetting {
  key: string;
  label: string;
  description: string;
  settingType: string;
  default: unknown;
  options: SettingOption[] | null;
}

export interface EngineCapabilities {
  gpu_supported: boolean;
  settings: EngineSetting[];
}

export interface StatusUpdatePayload {
  seq: number;
  status: string;
}

export type DictationStatus = 'Ready' | 'Recording' | 'Transcribing' | 'Processing' | 'Typing' | 'Error';

export type AppRoute = 'setup' | 'home' | 'status' | 'history' | 'settings' | 'ui-lab' | 'help';
