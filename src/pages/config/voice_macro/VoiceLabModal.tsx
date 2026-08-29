import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import {
  IconAdjustmentsHorizontal,
  IconPlayerPlay,
  IconLoader2,
  IconCheck,
  IconTrash,
  IconRefresh,
  IconSparkles,
  IconHistory,
  IconAlertTriangle,
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

type TabType = 'studio' | 'presets' | 'history';

const DEFAULT_MODELS: BaseVoiceModelInfo[] = [
  { id: 'piper-en_GB-northern_english_male-medium', label: '🇬🇧 Northern English Male (SAS Price / Tactical)', is_multi_speaker: false },
  { id: 'piper-en_GB-alan-medium', label: '🇬🇧 Alan (Cold British Commander / Dark Baritone)', is_multi_speaker: false },
  { id: 'piper-en_US-norman-medium', label: '🇺🇸 Norman (Deep American Baritone / Dispatcher)', is_multi_speaker: false },
  { id: 'piper-en_US-joe-medium', label: '🇺🇸 Joe (Gritty Older Combat Veteran)', is_multi_speaker: false },
  { id: 'piper-en_US-bryce-medium', label: '🇺🇸 Bryce (High-Energy / Commanding Operator)', is_multi_speaker: false },
  { id: 'piper-en_US-danny-low', label: '🇺🇸 Danny (Fast Tactical Field Operator)', is_multi_speaker: false },
  { id: 'piper-en_US-ryan-low', label: '🇺🇸 Ryan (Deep Male / Titan Base)', is_multi_speaker: false },
  { id: 'piper-en_US-amy-low', label: '🛸 Amy (Cyberpunk EVA / Clear Sci-Fi Female)', is_multi_speaker: false },
  { id: 'piper-en_GB-cori-medium', label: '🇬🇧 Cori (Expressive British Female)', is_multi_speaker: false },
  { id: 'piper-en_US-glados', label: '🤖 GLaDOS (Iconic Robotic Portal AI)', is_multi_speaker: false },
  { id: 'piper-en_GB-southern_english_female-low', label: '✈️ Southern English Female (Flight Deck ATC)', is_multi_speaker: false },
  { id: 'piper-en_US-libritts_r-medium', label: '🎭 LibriTTS-R Multi-Speaker (904 Speakers)', is_multi_speaker: true },
];

const OPENING_CHIMES = [
  { value: 'none', label: 'None' },
  { value: 'tactical_double_beep', label: 'Tactical Double Beep (CS style)' },
  { value: 'radio_click', label: 'Subtle Radio Key-Click' },
  { value: 'cockpit_chime', label: 'Sci-Fi Cockpit Chime' },
  { value: 'transmit_blip', label: 'Military Transmit Blip' },
];

const CLOSING_CHIMES = [
  { value: 'none', label: 'None' },
  { value: 'radio_squelch', label: 'Subtle Radio Squelch' },
  { value: 'cs_radio_off', label: 'CS Comms Release Burst' },
  { value: 'mic_release_click', label: 'Mic Release Click' },
  { value: 'cockpit_ack', label: 'Two-Tone Cockpit Ack' },
];

export function VoiceLabModal({ onClose, onPresetSaved }: VoiceLabModalProps) {
  const activeTab = useSignal<TabType>('studio');
  const models = useSignal<BaseVoiceModelInfo[]>(DEFAULT_MODELS);
  const savedPresets = useSignal<VoicePreset[]>([]);

  // Studio Form State
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

  const presetName = useSignal('');
  const isPreviewing = useSignal(false);
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

  const handlePreview = async () => {
    if (isPreviewing.value) return;
    previewError.value = null;
    isPreviewing.value = true;

    try {
      await invoke('preview_custom_tts_voice', {
        params: {
          text: text.value.trim() || 'Command confirmed',
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
        },
      });
    } catch (e) {
      previewError.value = String(e);
    } finally {
      isPreviewing.value = false;
    }
  };

  const handleSavePreset = async () => {
    const name = presetName.value.trim();
    if (!name) return;

    const id = `preset-${name.toLowerCase().replace(/[^a-z0-9]/g, '-')}`;
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
      savedPresets.value = [...savedPresets.value.filter((p) => p.id !== id), newPreset];
      saveSuccessMsg.value = `Saved "${name}"!`;
      presetName.value = '';
      onPresetSaved?.(newPreset);
      setTimeout(() => {
        saveSuccessMsg.value = null;
      }, 3000);
    } catch (e) {
      previewError.value = `Failed to save preset: ${e}`;
    }
  };

  const handleLoadPreset = (p: VoicePreset) => {
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
    } catch (e) {
      previewError.value = `Failed to delete preset: ${e}`;
    }
  };

  const isMultiSpeaker = modelKey.value.includes('libritts');

  return (
    <Modal
      title="🎙️ Voice Studio — Custom Voice Designer"
      onClose={onClose}
      footer={
        <Button
          variant="ghost"
          pill
          onClick={onClose}
          style={{ padding: '6px 16px', fontSize: '12px' }}
        >
          Close
        </Button>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', flex: 1, minHeight: 0 }}>
        {/* Navigation Sub-Tabs */}
        <div style={{ display: 'flex', gap: '6px', borderBottom: '1px solid rgba(255, 255, 255, 0.08)', paddingBottom: '8px', flexShrink: 0 }}>
          <Button
            variant={activeTab.value === 'studio' ? 'primary' : 'configAction'}
            onClick={() => { activeTab.value = 'studio'; }}
            style={{ fontSize: '11.5px', padding: '5px 12px', display: 'flex', alignItems: 'center', gap: '5px' }}
          >
            <IconAdjustmentsHorizontal size={13} />
            <span>Interactive Studio</span>
          </Button>

          <Button
            variant={activeTab.value === 'presets' ? 'primary' : 'configAction'}
            onClick={() => { activeTab.value = 'presets'; }}
            style={{ fontSize: '11.5px', padding: '5px 12px', display: 'flex', alignItems: 'center', gap: '5px' }}
          >
            <IconSparkles size={13} />
            <span>Saved Presets ({savedPresets.value.length})</span>
          </Button>

          <Button
            variant={activeTab.value === 'history' ? 'primary' : 'configAction'}
            onClick={() => { activeTab.value = 'history'; }}
            style={{ fontSize: '11.5px', padding: '5px 12px', display: 'flex', alignItems: 'center', gap: '5px' }}
          >
            <IconHistory size={13} />
            <span>Feedback Log (1–27)</span>
          </Button>
        </div>

        {/* TAB 1: INTERACTIVE STUDIO */}
        {activeTab.value === 'studio' && (
          <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 0.9fr', gap: '14px', flex: 1, minHeight: 0, overflowY: 'auto', paddingRight: '2px' }}>
            {/* Left Column: Voice & Phrase */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
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

              {isMultiSpeaker && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', background: 'rgba(99, 102, 241, 0.08)', padding: '8px', borderRadius: '6px' }}>
                  <label style={{ fontSize: '11px', fontWeight: 600, color: '#c7d2fe' }}>
                    Speaker ID (0 to 903): {speakerId.value}
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="903"
                    step="1"
                    value={speakerId.value}
                    onInput={(e) => { speakerId.value = parseInt((e.target as HTMLInputElement).value, 10) || 0; }}
                    style={{ accentColor: tokens.colors.accentPrimary, cursor: 'pointer' }}
                  />
                  <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
                    Tip: Try ID 700 (Heavy Titan) or ID 200 (Operations Dispatcher).
                  </span>
                </div>
              )}

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

              {/* Quick Preset Chips */}
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px' }}>
                <button type="button" onClick={() => { text.value = 'Titan online. Core temperature nominal. Weapons combat ready.'; }} style={chipStyle}>⚔️ Titan</button>
                <button type="button" onClick={() => { text.value = 'Bravo Six, going dark. Target neutralized, requesting exfil.'; }} style={chipStyle}>📻 SAS Price</button>
                <button type="button" onClick={() => { text.value = 'Target down! Air strike inbound on marked coordinates! Heads down!'; }} style={chipStyle}>🔥 Shouting</button>
                <button type="button" onClick={() => { text.value = 'Maximum armor engaged. Energy levels at one hundred percent.'; }} style={chipStyle}>🛡️ Nanosuit</button>
                <button type="button" onClick={() => { text.value = 'All automated workflows completed successfully.'; }} style={chipStyle}>🎙️ Studio</button>
              </div>

              {/* Preview Button */}
              <div style={{ marginTop: 'auto', paddingTop: '8px' }}>
                <Button
                  variant="primary"
                  onClick={handlePreview}
                  disabled={isPreviewing.value}
                  style={{ width: '100%', padding: '8px 14px', fontSize: '12px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px' }}
                >
                  {isPreviewing.value ? (
                    <>
                      <IconLoader2 size={14} className="spin" />
                      <span>Generating Audio in Both Ears...</span>
                    </>
                  ) : (
                    <>
                      <IconPlayerPlay size={14} />
                      <span>▶ Preview Audio Feedback</span>
                    </>
                  )}
                </Button>
              </div>

              {/* Save Preset Card */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', background: 'rgba(255, 255, 255, 0.03)', padding: '8px', borderRadius: '6px', border: '1px solid rgba(255, 255, 255, 0.06)' }}>
                <label style={{ fontSize: '10.5px', fontWeight: 600, color: tokens.colors.textMuted, textTransform: 'uppercase' }}>
                  Save Custom Preset
                </label>
                <div style={{ display: 'flex', gap: '6px' }}>
                  <input
                    type="text"
                    value={presetName.value}
                    onInput={(e) => { presetName.value = (e.target as HTMLInputElement).value; }}
                    placeholder="Preset Name (e.g. British SAS Overlord)"
                    style={{ ...inputBaseStyle, fontSize: '11px', padding: '5px 8px', flex: 1 }}
                  />
                  <Button
                    variant="configAction"
                    onClick={handleSavePreset}
                    style={{ fontSize: '11px', padding: '5px 10px', color: '#34d399', borderColor: 'rgba(52, 211, 153, 0.4)' }}
                  >
                    <span>💾 Save</span>
                  </Button>
                </div>
                {saveSuccessMsg.value && (
                  <span style={{ fontSize: '10.5px', color: '#34d399', display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <IconCheck size={12} /> {saveSuccessMsg.value}
                  </span>
                )}
              </div>
            </div>

            {/* Right Column: DSP Controls */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', background: 'rgba(0, 0, 0, 0.25)', padding: '10px', borderRadius: '8px', border: '1px solid rgba(255, 255, 255, 0.06)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid rgba(255, 255, 255, 0.06)', paddingBottom: '4px' }}>
                <span style={{ fontSize: '11px', fontWeight: 700, color: '#38bdf8', textTransform: 'uppercase' }}>
                  Acoustic Tuning Knobs
                </span>
                <button
                  type="button"
                  onClick={handleResetDsp}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: '#a5b4fc',
                    fontSize: '10.5px',
                    fontWeight: 600,
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '3px',
                  }}
                >
                  <IconRefresh size={11} />
                  <span>Reset DSP</span>
                </button>
              </div>

              {/* Sliders */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10.5px', color: tokens.colors.textMuted }}>
                  <span>Pitch Shift</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{pitch.value > 0 ? `+${pitch.value}` : pitch.value} st</span>
                </div>
                <input type="range" min="-12" max="12" step="0.5" value={pitch.value} onInput={(e) => { pitch.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10.5px', color: tokens.colors.textMuted }}>
                  <span>Playback Speed</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{speed.value.toFixed(2)}x</span>
                </div>
                <input type="range" min="0.6" max="1.5" step="0.05" value={speed.value} onInput={(e) => { speed.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10.5px', color: tokens.colors.textMuted }}>
                  <span>Sub-Bass Weight (&lt;140Hz)</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{Math.round(subBass.value * 100)}%</span>
                </div>
                <input type="range" min="0" max="1" step="0.05" value={subBass.value} onInput={(e) => { subBass.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10.5px', color: tokens.colors.textMuted }}>
                  <span>Cockpit Comb Resonance</span>
                  <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{Math.round(combMix.value * 100)}%</span>
                </div>
                <input type="range" min="0" max="1" step="0.05" value={combMix.value} onInput={(e) => { combMix.value = parseFloat((e.target as HTMLInputElement).value); }} style={sliderStyle} />
              </div>

              {/* Chimes Dropdowns */}
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '6px', marginTop: '2px' }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                  <label style={{ fontSize: '10px', color: tokens.colors.textMuted }}>Opening Beep</label>
                  <SelectField
                    value={openingChime.value}
                    options={OPENING_CHIMES}
                    onChange={(val) => { openingChime.value = val; }}
                    ariaLabel="Opening Chime"
                    style={{ width: '100%', fontSize: '10.5px' }}
                  />
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                  <label style={{ fontSize: '10px', color: tokens.colors.textMuted }}>Closing Squelch</label>
                  <SelectField
                    value={closingChime.value}
                    options={CLOSING_CHIMES}
                    onChange={(val) => { closingChime.value = val; }}
                    ariaLabel="Closing Squelch"
                    style={{ width: '100%', fontSize: '10.5px' }}
                  />
                </div>
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
                <label htmlFor="vl_radio_check" style={{ fontSize: '10.5px', color: '#fff', cursor: 'pointer' }}>
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
          </div>
        )}

        {/* TAB 2: SAVED PRESETS */}
        {activeTab.value === 'presets' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1, minHeight: 0, overflowY: 'auto' }}>
            {savedPresets.value.length === 0 ? (
              <p style={{ color: tokens.colors.textMuted, fontSize: '12px', padding: '16px', textAlign: 'center' }}>
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
                  }}
                >
                  <div>
                    <div style={{ fontSize: '12.5px', fontWeight: 600, color: '#c7d2fe' }}>{p.name}</div>
                    <div style={{ fontSize: '10.5px', color: tokens.colors.textMuted }}>
                      {p.model_key} • Pitch: {p.pitch}st • Speed: {p.speed}x • Radio: {p.radio_bandpass ? 'Yes' : 'No'}
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: '6px' }}>
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

        {/* TAB 3: FEEDBACK LOG */}
        {activeTab.value === 'history' && (
          <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', fontSize: '11px', border: '1px solid rgba(255, 255, 255, 0.06)', borderRadius: '6px' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left' }}>
              <thead>
                <tr style={{ background: 'rgba(255, 255, 255, 0.04)', color: tokens.colors.textMuted }}>
                  <th style={{ padding: '6px 8px' }}>#</th>
                  <th style={{ padding: '6px 8px' }}>Name / File</th>
                  <th style={{ padding: '6px 8px' }}>Base Model</th>
                  <th style={{ padding: '6px 8px' }}>Status</th>
                  <th style={{ padding: '6px 8px' }}>Notes</th>
                </tr>
              </thead>
              <tbody>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#4</td><td>04_tactical_sas_price.wav</td><td>Northern English Male</td><td style={{ color: '#34d399', fontWeight: 600 }}>Keep</td><td>Sounds quite good, nice British tactical tone.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#7</td><td>07_tactical_fast_field_danny.wav</td><td>Danny</td><td style={{ color: '#34d399', fontWeight: 600 }}>Keep</td><td>Sounds pretty good as well.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#10</td><td>10_studio_cori_british.wav</td><td>Cori</td><td style={{ color: '#34d399', fontWeight: 600 }}>Keep</td><td>Sounds pretty good, distinct British tone.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#11</td><td>11_studio_amy_clean.wav</td><td>Amy</td><td style={{ color: '#34d399', fontWeight: 600 }}>Keep</td><td>Sounds pretty good.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#18</td><td>18_batman_alan_british_clean.wav</td><td>Alan</td><td style={{ color: '#34d399', fontWeight: 600 }}>Keep</td><td>Sounds really good! Very cold British voice.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#22</td><td>22_mech_libri_spk700_titan.wav</td><td>LibriTTS Spk 700</td><td style={{ color: '#38bdf8', fontWeight: 600 }}>Titan Baseline</td><td>Closest thing to a Titan style voice. Baseline for Titan.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#13</td><td>13_batman_norman_clean.wav</td><td>Norman</td><td style={{ color: '#38bdf8' }}>Narration</td><td>Good deep narration/dispatcher voice.</td></tr>
                <tr style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)' }}><td>#21</td><td>21_mech_libri_spk200_titan.wav</td><td>LibriTTS Spk 200</td><td style={{ color: '#38bdf8' }}>Dispatcher</td><td>Professional dispatcher voice.</td></tr>
              </tbody>
            </table>
          </div>
        )}

        {previewError.value && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px', color: '#f87171' }}>
            <IconAlertTriangle size={13} />
            <span>{previewError.value}</span>
          </div>
        )}
      </div>
    </Modal>
  );
}

const chipStyle = {
  background: 'rgba(255, 255, 255, 0.05)',
  border: '1px solid rgba(255, 255, 255, 0.08)',
  color: tokens.colors.textSecondary,
  fontSize: '10px',
  padding: '3px 6px',
  borderRadius: '4px',
  cursor: 'pointer',
};

const sliderStyle = {
  accentColor: tokens.colors.accentPrimary,
  cursor: 'pointer',
  height: '4px',
};
