import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import {
  IconPlayerPlay,
  IconPlayerStop,
  IconLoader2,
  IconCheck,
  IconTrash,
  IconRefresh,
  IconAlertTriangle,
  IconPlus,
} from '@tabler/icons-preact';
import { Modal } from '../../../components/Modal.tsx';
import { Button } from '../../../components/Button.tsx';
import { SelectField } from '../../../components/SelectField.tsx';
import type { BaseVoiceModelInfo, VoicePreset } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';

interface VoiceLabModalProps {
  onClose: () => void;
  onPresetSaved?: (preset: VoicePreset) => void;
}

type TabType = 'studio' | 'presets';

const DEFAULT_MODELS: BaseVoiceModelInfo[] = [
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

const CHIME_OPTIONS = [
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

export function VoiceLabModal({ onClose, onPresetSaved }: VoiceLabModalProps) {
  const activeTab = useSignal<TabType>('studio');
  const models = useSignal<BaseVoiceModelInfo[]>(DEFAULT_MODELS);
  const savedPresets = useSignal<VoicePreset[]>([]);
  const currentPresetId = useSignal<string | null>(null);

  // Studio Form State
  const presetName = useSignal('');
  const text = useSignal('Titan online. Core temperature nominal. All weapon systems combat ready.');
  const modelKey = useSignal('piper-en_GB-northern_english_male-medium');
  const speakerId = useSignal(700);
  const speed = useSignal(1.0);
  const noiseScale = useSignal(0.667);
  const pitch = useSignal(0.0);
  const subBass = useSignal(0.0);
  const combMix = useSignal(0.0);
  const flangerMix = useSignal(0.0);
  const radioBandpass = useSignal(false);
  const radioDrive = useSignal(2.2);
  const rfNoise = useSignal(0.25);
  const openingChime = useSignal('none');
  const closingChime = useSignal('none');

  const isPlaying = useSignal(false);
  const isSynthesizing = useSignal(false);
  const playTimer = useSignal<ReturnType<typeof setTimeout> | null>(null);
  const previewError = useSignal<string | null>(null);
  const saveSuccessMsg = useSignal<string | null>(null);

  useSignalEffect(() => {
    invoke<BaseVoiceModelInfo[]>('get_available_base_voice_models')
      .then((m) => {
        if (m && m.length > 0) models.value = m;
      })
      .catch(() => {});

    invoke<VoicePreset[]>('get_custom_voice_presets')
      .then((p) => {
        if (p) savedPresets.value = p;
      })
      .catch(() => {});
  });

  const handleResetDsp = () => {
    pitch.value = 0.0;
    speed.value = 1.0;
    noiseScale.value = 0.667;
    subBass.value = 0.0;
    combMix.value = 0.0;
    flangerMix.value = 0.0;
    openingChime.value = 'none';
    closingChime.value = 'none';
    radioBandpass.value = false;
    radioDrive.value = 2.2;
    rfNoise.value = 0.25;
  };

  const handleStartNewPreset = () => {
    currentPresetId.value = null;
    presetName.value = '';
    handleResetDsp();
  };

  const handleTogglePreview = async () => {
    if (isPlaying.value) {
      if (playTimer.value) clearTimeout(playTimer.value);
      isPlaying.value = false;
      try {
        await invoke('stop_macro_sound_playback');
      } catch (e) {
        console.warn('Failed to stop sound:', e);
      }
      return;
    }

    if (isSynthesizing.value) return;
    previewError.value = null;
    isSynthesizing.value = true;

    try {
      const res = await invoke<{ duration_secs: number }>('preview_custom_tts_voice', {
        params: {
          text: text.value.trim() || 'Command confirmed',
          model_key: modelKey.value,
          speaker_id: parseInt(String(speakerId.value), 10) || 0,
          speed: parseFloat(String(speed.value)) || 1.0,
          noise_scale: parseFloat(String(noiseScale.value)) || 0.667,
          pitch: parseFloat(String(pitch.value)) || 0.0,
          sub_bass: parseFloat(String(subBass.value)) || 0.0,
          comb_mix: parseFloat(String(combMix.value)) || 0.0,
          flanger_mix: parseFloat(String(flangerMix.value)) || 0.0,
          radio_bandpass: Boolean(radioBandpass.value),
          radio_drive: parseFloat(String(radioDrive.value)) || 1.0,
          rf_noise: parseFloat(String(rfNoise.value)) || 0.0,
          opening_chime: openingChime.value || 'none',
          closing_chime: closingChime.value || 'none',
        },
      });

      isSynthesizing.value = false;
      isPlaying.value = true;

      const durMs = Math.round(((res && res.duration_secs) || 3.0) * 1000) + 400;
      if (playTimer.value) clearTimeout(playTimer.value);
      playTimer.value = setTimeout(() => {
        isPlaying.value = false;
      }, durMs);
    } catch (e) {
      console.error('Preview custom voice failed:', e);
      previewError.value = String(e);
      isSynthesizing.value = false;
      isPlaying.value = false;
    }
  };

  const handleSavePreset = async () => {
    const name = presetName.value.trim();
    if (!name) return;

    const id = currentPresetId.value || `preset-${Date.now()}`;
    const newPreset: VoicePreset = {
      id,
      name,
      category: 'Custom Presets',
      description: `Voice Studio preset with ${modelKey.value}`,
      model_key: modelKey.value,
      speaker_id: speakerId.value,
      speed: speed.value,
      noise_scale: noiseScale.value,
      pitch: pitch.value,
      sub_bass: subBass.value,
      comb_mix: combMix.value,
      flanger_mix: flangerMix.value,
      radio_bandpass: radioBandpass.value,
      radio_drive: radioDrive.value,
      rf_noise: rfNoise.value,
      opening_chime: openingChime.value,
      closing_chime: closingChime.value,
    };

    try {
      await invoke('save_custom_voice_preset', { preset: newPreset });
      currentPresetId.value = id;
      savedPresets.value = [...savedPresets.value.filter((p) => p.id !== id), newPreset];
      saveSuccessMsg.value = `Saved "${name}"!`;
      onPresetSaved?.(newPreset);
      setTimeout(() => {
        saveSuccessMsg.value = null;
      }, 3000);
    } catch (e) {
      previewError.value = `Failed to save preset: ${e}`;
    }
  };

  const handleLoadPreset = (p: VoicePreset) => {
    currentPresetId.value = p.id;
    presetName.value = p.name;
    modelKey.value = p.model_key;
    speakerId.value = p.speaker_id || 0;
    speed.value = p.speed;
    pitch.value = p.pitch;
    subBass.value = p.sub_bass || 0;
    combMix.value = p.comb_mix || 0;
    flangerMix.value = p.flanger_mix || 0;
    radioBandpass.value = p.radio_bandpass || false;
    radioDrive.value = p.radio_drive || 1.0;
    rfNoise.value = p.rf_noise || 0;
    openingChime.value = p.opening_chime || 'none';
    closingChime.value = p.closing_chime || 'none';
    activeTab.value = 'studio';
  };

  const handleDeletePreset = async (presetId: string) => {
    try {
      await invoke('delete_custom_voice_preset', { presetId });
      savedPresets.value = savedPresets.value.filter((p) => p.id !== presetId);
      if (currentPresetId.value === presetId) {
        currentPresetId.value = null;
        presetName.value = '';
      }
    } catch (e) {
      previewError.value = `Failed to delete preset: ${e}`;
    }
  };

  const isMultiSpeaker = modelKey.value.includes('libritts');

  const tabs: { id: TabType; label: string }[] = [
    {
      id: 'studio',
      label: 'Studio',
    },
    {
      id: 'presets',
      label: savedPresets.value.length > 0 ? `Presets (${savedPresets.value.length})` : 'Presets',
    },
  ];

  return (
    <Modal
      onClose={onClose}
      showCloseButton={false}
      maxWidth="540px"
      topBar={
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(2, 1fr)',
            gap: '4px',
            background: 'rgba(0, 0, 0, 0.25)',
            padding: '3px',
            borderRadius: '999px',
            border: '1px solid rgba(255, 255, 255, 0.08)',
          }}
        >
          {tabs.map((tab) => {
            const isActive = activeTab.value === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => {
                  activeTab.value = tab.id;
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: '6px 12px',
                  borderRadius: '999px',
                  background: isActive
                    ? 'linear-gradient(135deg, rgba(88, 101, 242, 0.35) 0%, rgba(129, 140, 248, 0.2) 100%)'
                    : 'transparent',
                  border: isActive
                    ? '1px solid rgba(99, 102, 241, 0.55)'
                    : '1px solid transparent',
                  color: isActive ? '#ffffff' : tokens.colors.textSecondary,
                  fontSize: '12px',
                  fontWeight: isActive ? 600 : 500,
                  cursor: 'pointer',
                  transition: tokens.transitions.fast,
                  whiteSpace: 'nowrap',
                }}
              >
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>
      }
      footer={
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            width: '100%',
            gap: '8px',
          }}
        >
          <div>
            {saveSuccessMsg.value && (
              <span
                style={{
                  fontSize: '11px',
                  color: tokens.colors.success,
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                }}
              >
                <IconCheck size={13} /> {saveSuccessMsg.value}
              </span>
            )}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Button
              variant="ghost"
              onClick={onClose}
              style={{ padding: '6px 14px', fontSize: '12px' }}
            >
              Close
            </Button>

            {activeTab.value === 'studio' && (
              <Button
                variant="configAction"
                onClick={handleSavePreset}
                disabled={!presetName.value.trim()}
                style={{
                  padding: '6px 16px',
                  fontSize: '12px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                }}
              >
                <IconCheck size={14} />
                <span>{currentPresetId.value ? 'Update Preset' : 'Save Preset'}</span>
              </Button>
            )}
          </div>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', width: '100%' }}>
        {/* TAB 1: INTERACTIVE STUDIO (Vertical Flow) */}
        {activeTab.value === 'studio' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', width: '100%' }}>
            {/* Preset Name Field (At the top of form) */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    color: tokens.colors.textMuted,
                    textTransform: 'uppercase',
                    letterSpacing: '0.4px',
                  }}
                >
                  Preset Name
                </label>
                {currentPresetId.value && (
                  <button
                    type="button"
                    onClick={handleStartNewPreset}
                    style={{
                      background: 'none',
                      border: 'none',
                      color: tokens.colors.accentPrimary,
                      fontSize: '10.5px',
                      fontWeight: 600,
                      cursor: 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '3px',
                      padding: 0,
                    }}
                    title="Clear current preset and start a new unlinked voice"
                  >
                    <IconPlus size={11} />
                    <span>New Preset</span>
                  </button>
                )}
              </div>
              <input
                type="text"
                value={presetName.value}
                onInput={(e) => {
                  presetName.value = (e.target as HTMLInputElement).value;
                }}
                placeholder="e.g. British SAS Overlord, Cockpit AI"
                style={{ ...inputBaseStyle, padding: '8px 10px', fontSize: '12px' }}
              />
            </div>

            {/* Voice Model Selection */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <label style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted, textTransform: 'uppercase' }}>
                Base Neural Voice Model
              </label>
              <SelectField
                value={modelKey.value}
                options={models.value.map((m) => ({ value: m.id, label: m.label }))}
                onChange={(val) => { modelKey.value = val; }}
                ariaLabel="Base Voice Model"
                style={{ width: '100%', fontSize: '11.5px' }}
              />
            </div>

            {/* LibriTTS Multi-Speaker Slider */}
            {isMultiSpeaker && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', background: 'rgba(99, 102, 241, 0.08)', padding: '10px 12px', borderRadius: '8px', border: '1px solid rgba(99, 102, 241, 0.2)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', fontWeight: 600, color: '#c7d2fe' }}>
                  <span>Speaker ID (0 to 903)</span>
                  <span style={{ fontFamily: 'monospace' }}>{speakerId.value}</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="903"
                  step="1"
                  value={speakerId.value}
                  onInput={(e) => { speakerId.value = parseInt((e.target as HTMLInputElement).value, 10) || 0; }}
                  style={sliderStyle}
                />
                <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
                  Tip: Try ID 700 (Heavy Titan) or ID 200 (Operations Dispatcher).
                </span>
              </div>
            )}

            {/* Spoken Test Phrase */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <label style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted, textTransform: 'uppercase' }}>
                Spoken Test Phrase
              </label>
              <textarea
                value={text.value}
                onInput={(e) => { text.value = (e.target as HTMLTextAreaElement).value; }}
                rows={2}
                style={{ ...inputBaseStyle, fontSize: '11.5px', padding: '7px 9px', resize: 'vertical' }}
              />
            </div>

            {/* DSP Controls Card */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', background: 'rgba(0, 0, 0, 0.25)', padding: '12px 14px', borderRadius: '8px', border: '1px solid rgba(255, 255, 255, 0.06)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid rgba(255, 255, 255, 0.06)', paddingBottom: '6px' }}>
                <span style={{ fontSize: '11px', fontWeight: 700, color: tokens.colors.textSecondary, textTransform: 'uppercase', letterSpacing: '0.4px' }}>
                  Acoustic Tuning Knobs
                </span>
                <button
                  type="button"
                  onClick={handleResetDsp}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: tokens.colors.textMuted,
                    fontSize: '10.5px',
                    fontWeight: 600,
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '3px',
                    transition: tokens.transitions.fast,
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = '#ffffff';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = tokens.colors.textMuted;
                  }}
                >
                  <IconRefresh size={11} />
                  <span>Reset</span>
                </button>
              </div>

              {/* Sliders with proper vertical clearance */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: tokens.colors.textMuted }}>
                  <span>Pitch Shift</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{pitch.value > 0 ? `+${pitch.value}` : pitch.value} st</span>
                </div>
                <input type="range" min="-12" max="12" step="0.5" value={pitch.value} onInput={(e) => { pitch.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: tokens.colors.textMuted }}>
                  <span>Playback Speed</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{speed.value.toFixed(2)}x</span>
                </div>
                <input type="range" min="0.6" max="1.5" step="0.05" value={speed.value} onInput={(e) => { speed.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: tokens.colors.textMuted }}>
                  <span>Sub-Bass Weight (&lt;140Hz)</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{Math.round(subBass.value * 100)}%</span>
                </div>
                <input type="range" min="0" max="1" step="0.05" value={subBass.value} onInput={(e) => { subBass.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: tokens.colors.textMuted }}>
                  <span>Cockpit Comb Resonance</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{Math.round(combMix.value * 100)}%</span>
                </div>
                <input type="range" min="0" max="1" step="0.05" value={combMix.value} onInput={(e) => { combMix.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              {/* Chimes Dropdowns (Stacked vertically) */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted }}>Opening Sound Effect</label>
                <SelectField
                  value={openingChime.value}
                  options={CHIME_OPTIONS}
                  onChange={(val) => { openingChime.value = val; }}
                  ariaLabel="Opening Sound Effect"
                  style={{ width: '100%', fontSize: '11.5px' }}
                />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted }}>Closing Sound Effect</label>
                <SelectField
                  value={closingChime.value}
                  options={CHIME_OPTIONS}
                  onChange={(val) => { closingChime.value = val; }}
                  ariaLabel="Closing Sound Effect"
                  style={{ width: '100%', fontSize: '11.5px' }}
                />
              </div>

              {/* Radio Bandpass Toggle */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginTop: '4px' }}>
                <input
                  type="checkbox"
                  id="vl_radio_check"
                  checked={radioBandpass.value}
                  onChange={(e) => { radioBandpass.value = (e.target as HTMLInputElement).checked; }}
                  style={{ accentColor: tokens.colors.accentPrimary, cursor: 'pointer' }}
                />
                <label htmlFor="vl_radio_check" style={{ fontSize: '10.5px', color: tokens.colors.textPrimary, cursor: 'pointer' }}>
                  Military VHF Bandpass (420Hz – 3.4kHz)
                </label>
              </div>

              {radioBandpass.value && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', paddingLeft: '14px' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: tokens.colors.textMuted }}>
                    <span>Radio Drive: {radioDrive.value.toFixed(1)}x</span>
                  </div>
                  <input type="range" min="1.0" max="4.0" step="0.2" value={radioDrive.value} onInput={(e) => { radioDrive.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
                </div>
              )}
            </div>

            {/* Preview / Stop Button */}
            <div>
              <Button
                variant={isPlaying.value ? 'danger' : 'primary'}
                onClick={handleTogglePreview}
                disabled={isSynthesizing.value}
                style={{
                  width: '100%',
                  padding: '8px 14px',
                  fontSize: '12px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '6px',
                  ...(isPlaying.value
                    ? {
                        background: 'rgba(239, 68, 68, 0.3)',
                        borderColor: '#ef4444',
                        color: '#fca5a5',
                      }
                    : {}),
                }}
              >
                {isSynthesizing.value ? (
                  <>
                    <IconLoader2 size={14} className="spin" />
                    <span>Rendering Audio...</span>
                  </>
                ) : isPlaying.value ? (
                  <>
                    <IconPlayerStop size={14} />
                    <span>Stop Audio Preview</span>
                  </>
                ) : (
                  <>
                    <IconPlayerPlay size={14} />
                    <span>Preview Audio Feedback</span>
                  </>
                )}
              </Button>
            </div>
          </div>
        )}

        {/* TAB 2: SAVED PRESETS */}
        {activeTab.value === 'presets' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', width: '100%' }}>
            {savedPresets.value.length === 0 ? (
              <p style={{ color: tokens.colors.textMuted, fontSize: '12px', padding: '24px', textAlign: 'center' }}>
                No custom presets saved yet. Tune a voice in the Studio and click Save!
              </p>
            ) : (
              savedPresets.value.map((p) => (
                <div
                  key={p.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 12px',
                    borderRadius: '6px',
                    background: 'rgba(255, 255, 255, 0.03)',
                    border: '1px solid rgba(255, 255, 255, 0.06)',
                    gap: '8px',
                  }}
                >
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div style={{ fontSize: '12px', fontWeight: 600, color: '#c7d2fe', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{p.name}</div>
                    <div style={{ fontSize: '10.5px', color: tokens.colors.textMuted, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                      {p.model_key} • Pitch: {p.pitch}st • Speed: {p.speed}x • Radio: {p.radio_bandpass ? 'Yes' : 'No'}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: '6px', flexShrink: 0 }}>
                    <Button variant="primary" onClick={() => handleLoadPreset(p)} style={{ fontSize: '10.5px', padding: '4px 8px' }}>
                      Load in Studio
                    </Button>
                    <Button variant="danger" onClick={() => handleDeletePreset(p.id)} style={{ fontSize: '10.5px', padding: '4px 8px' }}>
                      <IconTrash size={12} />
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {previewError.value && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px', color: '#f87171', flexShrink: 0 }}>
            <IconAlertTriangle size={13} />
            <span>{previewError.value}</span>
          </div>
        )}
      </div>
    </Modal>
  );
}

const sliderStyle = {
  accentColor: tokens.colors.accentPrimary,
  cursor: 'pointer',
  height: '5px',
  margin: '6px 0 2px 0',
  width: '100%',
};
