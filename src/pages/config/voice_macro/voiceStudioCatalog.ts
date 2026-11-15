import type { BaseVoiceModelInfo } from '../../../types.ts';

/// Shown until the Python runner reports its voices. Duplicates the
/// runner's catalog; see get_available_base_voice_models.
export const FALLBACK_VOICE_MODELS: BaseVoiceModelInfo[] = [
  { id: 'piper-en_GB-northern_english_male-medium', label: 'Northern English Male (SAS Price / Tactical)', is_multi_speaker: false },
  { id: 'piper-en_GB-alan-medium', label: 'Alan (Cold British Commander / Dark Baritone)', is_multi_speaker: false },
  { id: 'piper-en_US-norman-medium', label: 'Norman (Deep American Baritone / Dispatcher)', is_multi_speaker: false },
  { id: 'piper-en_US-joe-medium', label: 'Joe (Gritty Older Combat Veteran)', is_multi_speaker: false },
  { id: 'piper-en_US-bryce-medium', label: 'Bryce (High-Energy / Commanding Operator)', is_multi_speaker: false },
  { id: 'piper-en_US-danny-low', label: 'Danny (Fast Tactical Field Operator)', is_multi_speaker: false },
  { id: 'piper-en_US-ryan-low', label: 'Ryan (Deep Male / Titan Base)', is_multi_speaker: false },
  { id: 'piper-en_US-amy-low', label: 'Amy (Cyberpunk EVA / Clear Sci-Fi Female)', is_multi_speaker: false },
  { id: 'piper-en_GB-cori-medium', label: 'Cori (Expressive British Female)', is_multi_speaker: false },
  { id: 'piper-en_US-glados', label: 'GLaDOS (Iconic Robotic Portal AI)', is_multi_speaker: false },
  { id: 'piper-en_GB-southern_english_female-low', label: 'Southern English Female (Flight Deck ATC)', is_multi_speaker: false },
  { id: 'piper-en_US-libritts_r-medium', label: 'LibriTTS-R Multi-Speaker (904 Speakers)', is_multi_speaker: true },
];

export const CHIME_OPTIONS = [
  { value: 'none', label: 'None' },
  { value: 'tactical_double_beep', label: 'Tactical Double Beep (CS style)' },
  { value: 'radio_click', label: 'Subtle Radio Key-Click' },
  { value: 'cockpit_chime', label: 'Sci-Fi Cockpit Chime' },
  { value: 'transmit_blip', label: 'Military Transmit Blip' },
  { value: 'radio_squelch', label: 'Subtle Radio Squelch' },
  { value: 'cs_radio_off', label: 'CS Comms Release Burst' },
  { value: 'mic_release_click', label: 'Mic Release Click' },
  { value: 'cockpit_ack', label: 'Two-Tone Cockpit Ack' },
];
