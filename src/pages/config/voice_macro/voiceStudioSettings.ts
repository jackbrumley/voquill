import type { VoicePreset } from '../../../types.ts';

/** Every adjustable part of a Voice Studio voice, named as the backend names them. */
export interface VoiceSettings {
  model_key: string;
  speaker_id: number;
  speed: number;
  noise_scale: number;
  pitch: number;
  sub_bass: number;
  comb_mix: number;
  flanger_mix: number;
  radio_bandpass: boolean;
  radio_drive: number;
  rf_noise: number;
  opening_chime: string;
  closing_chime: string;
}

export const DEFAULT_VOICE_SETTINGS: VoiceSettings = {
  model_key: 'piper-en_GB-northern_english_male-medium',
  speaker_id: 700,
  speed: 1.0,
  noise_scale: 0.667,
  pitch: 0.0,
  sub_bass: 0.0,
  comb_mix: 0.0,
  flanger_mix: 0.0,
  radio_bandpass: false,
  radio_drive: 1.0,
  rf_noise: 0.0,
  opening_chime: 'none',
  closing_chime: 'none',
};

/** Radio drive and noise applied when the radio effect is switched on fresh. */
export const RADIO_ON_DEFAULTS = { radio_drive: 2.2, rf_noise: 0.25 } as const;

/** Clears the tuning while keeping the chosen voice and speaker. */
export function resetTuning(settings: VoiceSettings): VoiceSettings {
  return { ...DEFAULT_VOICE_SETTINGS, model_key: settings.model_key, speaker_id: settings.speaker_id };
}

/** Radio drive and noise only apply while the radio effect is on. */
function withRadioApplied(settings: VoiceSettings): VoiceSettings {
  return settings.radio_bandpass
    ? settings
    : { ...settings, radio_drive: DEFAULT_VOICE_SETTINGS.radio_drive, rf_noise: DEFAULT_VOICE_SETTINGS.rf_noise };
}

export function settingsFromPreset(preset: VoicePreset): VoiceSettings {
  return {
    model_key: preset.model_key,
    speaker_id: preset.speaker_id ?? DEFAULT_VOICE_SETTINGS.speaker_id,
    speed: preset.speed,
    noise_scale: preset.noise_scale ?? DEFAULT_VOICE_SETTINGS.noise_scale,
    pitch: preset.pitch,
    sub_bass: preset.sub_bass ?? 0,
    comb_mix: preset.comb_mix ?? 0,
    flanger_mix: preset.flanger_mix ?? 0,
    radio_bandpass: preset.radio_bandpass ?? false,
    radio_drive: preset.radio_drive ?? DEFAULT_VOICE_SETTINGS.radio_drive,
    rf_noise: preset.rf_noise ?? 0,
    opening_chime: preset.opening_chime || 'none',
    closing_chime: preset.closing_chime || 'none',
  };
}

export function presetFromSettings(settings: VoiceSettings, id: string, name: string): VoicePreset {
  return {
    ...withRadioApplied(settings),
    id,
    name,
    category: 'Custom Presets',
    description: `Voice Studio preset with ${settings.model_key}`,
  };
}

/** The `preview_custom_tts_voice` parameters for these settings. */
export function previewParams(settings: VoiceSettings, text: string) {
  return { ...withRadioApplied(settings), text: text.trim() || 'Command confirmed' };
}
