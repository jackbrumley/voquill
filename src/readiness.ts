import type { AudioDevice, Config, LinuxPermissions, ModelInfo } from './types.ts';

/// Placeholder written by `persistConfig` whenever the API key field is
/// empty, so an empty key never persists to disk.
export const API_KEY_PLACEHOLDER = 'your_api_key_here';

export type AudioDeviceIssue = 'none' | 'no-devices' | 'device-missing';

/// Snapshot of everything that decides whether the app can be used right now
/// ("could someone press the hotkey and transcribe this second?"). The single
/// source of truth for the initial-setup gate and its per-row status.
export interface ReadinessStatus {
  isPermissionsReady: boolean;
  audioDeviceIssue: AudioDeviceIssue;
  isAudioDeviceReady: boolean;
  isTranscriptionReady: boolean;
  isHotkeyReady: boolean;
  isAllReady: boolean;
  /// Warn-only: dictation works, but local post-processing would fail.
  postProcessModelMissing: boolean;
}

export interface ReadinessInputs {
  permissions: LinuxPermissions | null;
  hotkeyError: string | null;
  availableMics: AudioDevice[];
  config: Config;
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
}

/// Pure derivation, computed fresh during render from live probe state.
export function computeReadiness(inputs: ReadinessInputs): ReadinessStatus {
  const { permissions, hotkeyError, availableMics, config, availableModels, modelStatus } = inputs;

  const isPermissionsReady = !!permissions
    && permissions.audio
    && permissions.shortcuts
    && permissions.input_emulation;

  const realMics = availableMics.filter((mic) => !mic.is_system_default);
  const configuredDevice = config.audio_device || 'default';
  const configuredDevicePresent = configuredDevice === 'default'
    || realMics.some((mic) => mic.id === configuredDevice);
  const audioDeviceIssue: AudioDeviceIssue = realMics.length === 0
    ? 'no-devices'
    : !configuredDevicePresent
      ? 'device-missing'
      : 'none';
  const isAudioDeviceReady = audioDeviceIssue === 'none';

  // The (engine, model) pairing must exist in the catalog AND be downloaded,
  // so a stale pairing (e.g. engine switched without a matching model) gates.
  const isTranscriptionReady = config.transcription_mode === 'Local'
    ? availableModels.some(
        (model) => model.engine === config.local_engine
          && model.size === config.local_model_size
          && !!modelStatus[model.size],
      )
    : config.openai_api_key.trim().length > 0 && config.openai_api_key !== API_KEY_PLACEHOLDER;

  const isHotkeyReady = hotkeyError === null;

  const postProcessModelMissing = config.post_process_enabled
    && config.post_process_provider === 'Local'
    && !modelStatus[config.post_process_model];

  return {
    isPermissionsReady,
    audioDeviceIssue,
    isAudioDeviceReady,
    isTranscriptionReady,
    isHotkeyReady,
    isAllReady: isPermissionsReady && isAudioDeviceReady && isTranscriptionReady && isHotkeyReady,
    postProcessModelMissing,
  };
}
