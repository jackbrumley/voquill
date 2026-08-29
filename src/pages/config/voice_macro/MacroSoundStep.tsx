import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  Icon,
  IconVolume,
  IconVolumeOff,
  IconMicrophone,
  IconUpload,
  IconPlayerPlay,
  IconPlayerStop,
  IconSparkles,
  IconLoader2,
  IconCheck,
  IconAlertTriangle,
  IconAdjustmentsHorizontal,
} from '@tabler/icons-preact';
import type { MacroSoundMode, VoicePersonaInfo } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { Button } from '../../../components/Button.tsx';
import { SelectField } from '../../../components/SelectField.tsx';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { VoiceLabModal } from './VoiceLabModal.tsx';

interface MacroSoundStepProps {
  macroId: string;
  soundMode: MacroSoundMode;
  onSoundModeChange: (mode: MacroSoundMode) => void;
  ttsText: string;
  onTtsTextChange: (text: string) => void;
  ttsVoice: string;
  onTtsVoiceChange: (voice: string) => void;
  ttsSpeed: number;
  onTtsSpeedChange: (speed: number) => void;
  ttsEffect?: string;
  onTtsEffectChange?: (effect: string) => void;
  ttsPitch?: number;
  onTtsPitchChange?: (pitch: number) => void;
}

const DEFAULT_VOICES: VoicePersonaInfo[] = [
  {
    id: 'tactical-comms',
    name: 'Tactical Comms',
    persona: 'SAS Tactical Operator',
    category: 'Military & Tactical',
    engine: 'piper',
    description: 'Northern English military comms with authentic VHF bandpass filtering, tactical overdrive, and tail squelch.',
    is_ready: true,
    default_effect: 'radio',
    default_pitch: 0.0,
    default_speed: 1.05,
  },
  {
    id: 'titan-mech',
    name: 'Titan Mech',
    persona: 'Armored Cockpit AI',
    category: 'Sci-Fi & Gaming',
    engine: 'piper',
    description: 'Deep, authoritative pilot system with metallic armored chassis resonance and sub-bass weight.',
    is_ready: true,
    default_effect: 'mech',
    default_pitch: -4.0,
    default_speed: 0.95,
  },
  {
    id: 'nanosuit',
    name: 'Nanosuit AI',
    persona: 'Tactical Combat Exosuit',
    category: 'Sci-Fi & Gaming',
    engine: 'piper',
    description: 'Cybernetic combat armor system with synthetic pitch modulation and power-shield resonance.',
    is_ready: true,
    default_effect: 'mech',
    default_pitch: -2.0,
    default_speed: 1.0,
  },
  {
    id: 'glados',
    name: 'GLaDOS AI',
    persona: 'Iconic Robot AI',
    category: 'Sci-Fi & Gaming',
    engine: 'piper',
    description: 'Iconic robotic AI with distinctive robotic inflections and dry demeanor.',
    is_ready: true,
    default_effect: 'clean',
    default_pitch: 0.0,
    default_speed: 1.0,
  },
  {
    id: 'cyberpunk-eva',
    name: 'Cyberpunk EVA',
    persona: 'Holographic Ship AI',
    category: 'Sci-Fi & Gaming',
    engine: 'piper',
    description: 'Futuristic spacecraft computer with holographic bridge reflections and crystal air clarity.',
    is_ready: true,
    default_effect: 'eva',
    default_pitch: 1.0,
    default_speed: 1.0,
  },
  {
    id: 'flight-deck',
    name: 'Flight Deck ATC',
    persona: 'British Flight Controller',
    category: 'Aviation & Simulation',
    engine: 'piper',
    description: 'Crisp British aviation air traffic control / cockpit automated warning system.',
    is_ready: true,
    default_effect: 'flight_deck',
    default_pitch: 0.0,
    default_speed: 1.0,
  },
  {
    id: 'nova-studio',
    name: 'Nova Studio (Female)',
    persona: 'Clean Studio Female',
    category: 'Studio & Natural',
    engine: 'piper',
    description: 'Natural, crystal-clear studio narration voice with zero acoustic coloration.',
    is_ready: true,
    default_effect: 'clean',
    default_pitch: 0.0,
    default_speed: 1.0,
  },
  {
    id: 'apex-studio',
    name: 'Apex Studio (Male)',
    persona: 'Authoritative Studio Male',
    category: 'Studio & Natural',
    engine: 'piper',
    description: 'Clean, warm, and natural male studio voice for desktop automation and productivity.',
    is_ready: true,
    default_effect: 'clean',
    default_pitch: 0.0,
    default_speed: 1.0,
  },
];

const SPEED_OPTIONS = [
  { label: '0.8x (Slow)', value: 0.8 },
  { label: '0.9x', value: 0.9 },
  { label: '1.0x (Normal)', value: 1.0 },
  { label: '1.1x', value: 1.1 },
  { label: '1.2x (Fast)', value: 1.2 },
];

export function MacroSoundStep({
  macroId,
  soundMode,
  onSoundModeChange,
  ttsText,
  onTtsTextChange,
  ttsVoice,
  onTtsVoiceChange,
  ttsSpeed,
  onTtsSpeedChange,
  ttsEffect,
  onTtsEffectChange,
  ttsPitch,
  onTtsPitchChange,
}: MacroSoundStepProps) {
  const availableVoices = useSignal<VoicePersonaInfo[]>(DEFAULT_VOICES);
  const isPreviewing = useSignal(false);
  const previewError = useSignal<string | null>(null);
  const isRecordingMic = useSignal(false);
  const importedFileName = useSignal<string | null>(null);
  const isVoiceLabModalOpen = useSignal(false);

  const fetchVoices = () => {
    invoke<VoicePersonaInfo[]>('get_available_tts_voices')
      .then((voices) => {
        if (voices && voices.length > 0) {
          availableVoices.value = voices;
        }
      })
      .catch(() => {
        // Fallback to default presets
      });
  };

  useSignalEffect(() => {
    fetchVoices();
  });

  const activePersona =
    availableVoices.value.find((v) => v.id === ttsVoice) ||
    availableVoices.value.find((v) => v.id === 'titan-mech') ||
    DEFAULT_VOICES[0];

  const handleSelectVoice = (selectedId: string) => {
    onTtsVoiceChange(selectedId);
    const matched = availableVoices.value.find((v) => v.id === selectedId);
    if (matched) {
      if (matched.default_effect && onTtsEffectChange) {
        onTtsEffectChange(matched.default_effect);
      }
      if (matched.default_pitch !== undefined && matched.default_pitch !== null && onTtsPitchChange) {
        onTtsPitchChange(matched.default_pitch);
      }
      if (matched.default_speed !== undefined && matched.default_speed !== null) {
        onTtsSpeedChange(matched.default_speed);
      }
    }
  };

  const handleTestPreview = async () => {
    if (isPreviewing.value) return;
    previewError.value = null;
    isPreviewing.value = true;

    try {
      if (soundMode === 'default') {
        await invoke('test_voice_macro_sound');
      } else if (soundMode === 'tts') {
        const text = ttsText.trim() || 'Command confirmed';
        const voice = ttsVoice || 'titan-mech';
        await invoke('preview_tts_voice', {
          text,
          voiceId: voice,
          speed: ttsSpeed || activePersona.default_speed || 1.0,
          effect: ttsEffect || activePersona.default_effect || undefined,
          pitch: ttsPitch ?? activePersona.default_pitch ?? 0.0,
        });
      } else {
        await invoke('play_macro_sound_preview', { macroId });
      }
    } catch (e) {
      previewError.value = String(e);
    } finally {
      isPreviewing.value = false;
    }
  };

  const handleBrowseFile = async () => {
    previewError.value = null;
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [
          {
            name: 'Audio Files',
            extensions: ['mp3', 'wav', 'ogg', 'flac', 'm4a', 'aac', 'wma'],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        const filename = selected.split(/[\\/]/).pop() || selected;
        importedFileName.value = filename;
        await invoke('import_macro_audio_file', {
          macroId,
          sourcePath: selected,
        });
      }
    } catch (e) {
      previewError.value = `File import failed: ${e}`;
    }
  };

  const handleToggleMicRecord = async () => {
    if (isRecordingMic.value) {
      isRecordingMic.value = false;
      try {
        await invoke('stop_mic_test');
        await invoke('play_macro_sound_preview', { macroId });
      } catch (e) {
        previewError.value = `Recording stop failed: ${e}`;
      }
    } else {
      previewError.value = null;
      isRecordingMic.value = true;
      try {
        await invoke('start_mic_test');
      } catch (e) {
        isRecordingMic.value = false;
        previewError.value = `Microphone record failed: ${e}`;
      }
    }
  };

  const soundModeTabs: { id: MacroSoundMode; label: string; icon: Icon }[] = [
    { id: 'default', label: 'Chirp', icon: IconVolume },
    { id: 'tts', label: 'AI Voice', icon: IconSparkles },
    { id: 'custom_file', label: 'Audio File', icon: IconUpload },
    { id: 'mic_recording', label: 'Record Mic', icon: IconMicrophone },
    { id: 'none', label: 'Mute', icon: IconVolumeOff },
  ];

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        flex: 1,
        minHeight: 0,
        overflowY: 'auto',
        padding: '2px',
      }}
    >
      {/* Sound Mode Selector Tabs */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '6px',
          padding: '10px 12px',
          borderRadius: '8px',
          background: 'rgba(255, 255, 255, 0.03)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}
      >
        <span
          style={{
            fontSize: '11px',
            fontWeight: 700,
            color: '#a5b4fc',
            textTransform: 'uppercase',
            letterSpacing: '0.4px',
          }}
        >
          Spoken Audio Feedback Mode
        </span>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '4px' }}>
          {soundModeTabs.map((tab) => {
            const isActive = soundMode === tab.id;
            const IconComponent = tab.icon;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => onSoundModeChange(tab.id)}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '4px',
                  padding: '6px 4px',
                  borderRadius: '6px',
                  fontSize: '11px',
                  fontWeight: isActive ? 600 : 500,
                  background: isActive
                    ? 'rgba(99, 102, 241, 0.25)'
                    : 'rgba(255, 255, 255, 0.04)',
                  border: isActive
                    ? '1px solid rgba(99, 102, 241, 0.55)'
                    : '1px solid rgba(255, 255, 255, 0.08)',
                  color: isActive ? '#c7d2fe' : tokens.colors.textSecondary,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease',
                  textAlign: 'center',
                }}
              >
                <IconComponent size={14} />
                <span style={{ fontSize: '10px', whiteSpace: 'nowrap' }}>{tab.label}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Mode Specific Settings Card */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '10px',
          padding: '12px',
          borderRadius: '8px',
          background: 'rgba(255, 255, 255, 0.03)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}
      >
        {/* TTS Mode Details */}
        {soundMode === 'tts' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <label
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  color: tokens.colors.textMuted,
                  textTransform: 'uppercase',
                }}
              >
                Spoken Confirmation Phrase
              </label>
              <input
                type="text"
                value={ttsText}
                onInput={(e) => onTtsTextChange((e.target as HTMLInputElement).value)}
                placeholder="e.g. Airstrike inbound, landing gear deployed..."
                style={{ ...inputBaseStyle, padding: '7px 10px', fontSize: '12px' }}
              />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    color: tokens.colors.textMuted,
                    textTransform: 'uppercase',
                  }}
                >
                  AI Voice
                </label>
                <button
                  type="button"
                  onClick={() => {
                    isVoiceLabModalOpen.value = true;
                  }}
                  style={{
                    background: 'none',
                    border: 'none',
                    color: tokens.colors.textSecondary,
                    fontSize: '10.5px',
                    fontWeight: 600,
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    padding: '0 2px',
                    transition: tokens.transitions.fast,
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = '#ffffff';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLButtonElement).style.color = tokens.colors.textSecondary;
                  }}
                >
                  <IconAdjustmentsHorizontal size={12} color={tokens.colors.accentPrimary} />
                  <span>Custom Voice Studio</span>
                </button>
              </div>
              <SelectField
                value={ttsVoice || 'titan-mech'}
                options={availableVoices.value.map((v) => ({
                  value: v.id,
                  label: `${v.name} — ${v.persona}`,
                  searchText: `${v.name} ${v.persona} ${v.category} ${v.description}`,
                }))}
                onChange={(val) => handleSelectVoice(val)}
                ariaLabel="AI Voice Persona"
                style={{ width: '100%', fontSize: '12px' }}
              />

              {activePersona && (
                <div
                  style={{
                    padding: '6px 8px',
                    borderRadius: '5px',
                    background: 'rgba(99, 102, 241, 0.08)',
                    border: '1px solid rgba(99, 102, 241, 0.2)',
                    fontSize: '11px',
                    color: tokens.colors.textSecondary,
                    lineHeight: '1.4',
                  }}
                >
                  <span style={{ color: '#c7d2fe', fontWeight: 600 }}>{activePersona.category}:</span>{' '}
                  {activePersona.description}
                </div>
              )}
            </div>

            {/* Playback Speed */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <label
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  color: tokens.colors.textMuted,
                  textTransform: 'uppercase',
                }}
              >
                Playback Speed
              </label>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '4px' }}>
                {SPEED_OPTIONS.map((opt) => {
                  const currentSpeed = ttsSpeed || activePersona.default_speed || 1.0;
                  const isSelected = Math.abs(currentSpeed - opt.value) < 0.01;
                  return (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => onTtsSpeedChange(opt.value)}
                      style={{
                        padding: '4px 2px',
                        fontSize: '10px',
                        fontWeight: isSelected ? 600 : 500,
                        borderRadius: '4px',
                        background: isSelected
                          ? 'rgba(99, 102, 241, 0.25)'
                          : 'rgba(255, 255, 255, 0.04)',
                        border: isSelected
                          ? '1px solid rgba(99, 102, 241, 0.5)'
                          : '1px solid rgba(255, 255, 255, 0.08)',
                        color: isSelected ? '#c7d2fe' : tokens.colors.textSecondary,
                        cursor: 'pointer',
                        textAlign: 'center',
                      }}
                    >
                      {opt.value}x
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {/* Custom Audio File Mode */}
        {soundMode === 'custom_file' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={{ fontSize: '11.5px', color: tokens.colors.textSecondary }}>
              Upload any custom sound effect (.wav, .mp3, .ogg) to play when this macro triggers.
            </span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Button
                variant="configAction"
                onClick={handleBrowseFile}
                style={{
                  padding: '6px 12px',
                  fontSize: '11.5px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                }}
              >
                <IconUpload size={13} />
                <span>Choose Audio File...</span>
              </Button>

              {importedFileName.value && (
                <span
                  style={{
                    fontSize: '11.5px',
                    fontFamily: 'monospace',
                    color: '#34d399',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                  }}
                >
                  <IconCheck size={13} />
                  <span>{importedFileName.value}</span>
                </span>
              )}
            </div>
          </div>
        )}

        {/* Mic Recording Mode */}
        {soundMode === 'mic_recording' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <span style={{ fontSize: '11.5px', color: tokens.colors.textSecondary }}>
              Record a quick custom voice response from your microphone.
            </span>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Button
                variant={isRecordingMic.value ? 'primary' : 'configAction'}
                onClick={handleToggleMicRecord}
                style={{
                  padding: '6px 12px',
                  fontSize: '11.5px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                  ...(isRecordingMic.value
                    ? {
                        background: 'rgba(239, 68, 68, 0.3)',
                        borderColor: '#ef4444',
                        color: '#fca5a5',
                      }
                    : {}),
                }}
              >
                {isRecordingMic.value ? (
                  <>
                    <IconPlayerStop size={13} />
                    <span>Stop & Save Recording</span>
                  </>
                ) : (
                  <>
                    <IconMicrophone size={13} />
                    <span>Record Audio Clip</span>
                  </>
                )}
              </Button>
              <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
                {isRecordingMic.value ? 'Recording audio snippet...' : 'Click to record snippet'}
              </span>
            </div>
          </div>
        )}

        {/* Default Chirp Mode */}
        {soundMode === 'default' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <span style={{ fontSize: '12px', color: tokens.colors.textSecondary, fontWeight: 500 }}>
              Standard Radio Chirp
            </span>
            <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
              Plays a brief, subtle audio beep to confirm execution.
            </span>
          </div>
        )}

        {/* Mute Mode */}
        {soundMode === 'none' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <span style={{ fontSize: '12px', color: tokens.colors.textSecondary, fontWeight: 500 }}>
              Silent Execution
            </span>
            <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
              No audio feedback or spoken voice will play.
            </span>
          </div>
        )}

        {/* Preview Button */}
        {soundMode !== 'none' && (
          <div
            style={{
              paddingTop: '6px',
              borderTop: '1px solid rgba(255, 255, 255, 0.06)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
            }}
          >
            <Button
              variant="configAction"
              onClick={handleTestPreview}
              disabled={isPreviewing.value}
              style={{
                padding: '5px 12px',
                fontSize: '11.5px',
                display: 'flex',
                alignItems: 'center',
                gap: '5px',
              }}
            >
              {isPreviewing.value ? (
                <>
                  <IconLoader2 size={13} className="spin" />
                  <span>Generating Audio...</span>
                </>
              ) : (
                <>
                  <IconPlayerPlay size={13} />
                  <span>Preview Audio Feedback</span>
                </>
              )}
            </Button>
          </div>
        )}

        {previewError.value && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              fontSize: '11px',
              color: '#f87171',
            }}
          >
            <IconAlertTriangle size={13} />
            <span>{previewError.value}</span>
          </div>
        )}
      </div>

      {isVoiceLabModalOpen.value && (
        <VoiceLabModal
          onClose={() => {
            isVoiceLabModalOpen.value = false;
          }}
          onPresetSaved={(newPreset) => {
            fetchVoices();
            onTtsVoiceChange(newPreset.id);
            if (onTtsSpeedChange) onTtsSpeedChange(newPreset.speed);
            if (onTtsPitchChange) onTtsPitchChange(newPreset.pitch);
          }}
        />
      )}
    </div>
  );
}
