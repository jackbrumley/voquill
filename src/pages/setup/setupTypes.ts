/** Permission probe results the setup screen shows. */
export interface PlatformPermissions {
  audio: boolean;
  shortcuts: boolean;
  input_emulation: boolean;
  shortcuts_status: string;
  shortcuts_detail?: string;
}

export interface SystemShortcutContext {
  distro?: string;
  desktop?: string;
  settings_path: string;
}

/** The settings the setup screen reads and changes. */
export interface SetupConfig {
  transcription_mode: 'API' | 'Local';
  local_model_size: string;
  local_engine: string;
  hotkey: string;
  hotkey_mode: 'HoldToTalk' | 'Toggle';
  audio_device: string | null;
  input_sensitivity: number;
  openai_api_key: string;
  post_process_enabled: boolean;
  post_process_provider: 'Local' | 'API';
  post_process_model: string;
}

/** One speech engine choice offered by `get_setup_recommendation`. */
export interface SetupOption {
  engine: string;
  model_size: string;
  title: string;
  description: string;
  download_bytes: number;
}

/** Speech engine choices from the backend, best first. */
export interface SetupRecommendation {
  graphics_card: string | null;
  options: SetupOption[];
}

export type SetupStepId = 'permissions' | 'microphone' | 'speech' | 'try';
