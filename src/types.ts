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
  debug_mode: boolean;
  enable_recording_logs: boolean;
  input_sensitivity: number;
  output_method: 'Typewriter' | 'Clipboard';
  copy_on_typewriter: boolean;
  language: string;
  enable_gpu: boolean;
  post_roll_ms: number;
  hotkey_mode: 'HoldToTalk' | 'Toggle';
  max_recording_duration_minutes: number;
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
  text: string;
  timestamp: string;
}

export interface AudioDevice {
  id: string;
  label: string;
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
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  releaseUrl: string;
  notesUrl?: string;
}

export interface StatusUpdatePayload {
  seq: number;
  status: string;
}

export type AppRoute = 'setup' | 'status' | 'history' | 'settings' | 'ui-lab';