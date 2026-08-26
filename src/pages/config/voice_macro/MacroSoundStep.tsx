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
} from '@tabler/icons-preact';
import type { MacroSoundMode, VoicePersonaInfo } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { Button } from '../../../components/Button.tsx';
import { SelectField } from '../../../components/SelectField.tsx';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';

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
    id: 'piper-en_US-amy-low',
    name: 'Cyberpunk EVA',
    persona: 'Crisp Sci-Fi Female AI',
    category: 'Sci-Fi / Cockpit',
    engine: 'piper',
    description: 'Futuristic, calm, and intelligent ship computer voice.',
    is_ready: true,
  },
  {
    id: 'piper-en_US-glados',
    name: 'GLaDOS AI',
    persona: 'Iconic Robot AI',
    category: 'Robotic / AI',
    engine: 'piper',
    description: 'Iconic robotic AI with distinctive robotic inflections.',
    is_ready: true,
  },
  {
    id: 'piper-en_US-ryan-low',
    name: 'Titan Mech',
    persona: 'Deep Cockpit Male',
    category: 'Sci-Fi / Cockpit',
    engine: 'piper',
    description: 'Authoritative, deep mechanical pilot system voice.',
    is_ready: true,
  },
  {
    id: 'piper-en_GB-southern_english_female-low',
    name: 'Aero Cockpit',
    persona: 'British Flight Deck AI',
    category: 'Sci-Fi / Cockpit',
    engine: 'piper',
    description: 'Crisp British ATC / flight deck automated system.',
    is_ready: true,
  },
  {
    id: 'piper-en_US-arctic-medium',
    name: 'Tactical Radio',
    persona: 'Military Radio Comms',
    category: 'Tactical Military',
    engine: 'piper',
    description: 'Tactical, direct military comms channel tone.',
    is_ready: true,
  },
  {
    id: 'piper-en_US-lessac-low',
    name: 'Nova Studio',
    persona: 'Clear Studio Voice',
    category: 'Realistic / Studio',
    engine: 'piper',
    description: 'Clean, human-grade natural studio voice.',
    is_ready: true,
  },
];

const SPEED_OPTIONS = [
  { label: '0.8x (Slow)', value: 0.8 },
  { label: '0.9x', value: 0.9 },
  { label: '1.0x (Normal)', value: 1.0 },
  { label: '1.1x', value: 1.1 },
  { label: '1.2x (Fast)', value: 1.2 },
];

const EFFECT_OPTIONS = [
  { value: 'mech', label: 'Titan Mech (Deep Metallic Ring Mod)' },
  { value: 'radio', label: 'Tactical Radio (Comms Bandpass Filter)' },
  { value: 'eva', label: 'Cyberpunk EVA (Ship Bridge Reflection)' },
  { value: 'clean', label: 'Clean / Studio (Natural Output)' },
];

const PITCH_OPTIONS = [
  { value: -6, label: 'Very Deep (-6)' },
  { value: -4, label: 'Deep (-4)' },
  { value: -2, label: 'Low (-2)' },
  { value: 0, label: 'Default (0)' },
  { value: 2, label: 'High (+2)' },
  { value: 4, label: 'Bright (+4)' },
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
  ttsEffect = 'clean',
  onTtsEffectChange,
  ttsPitch = 0,
  onTtsPitchChange,
}: MacroSoundStepProps) {
  const availableVoices = useSignal<VoicePersonaInfo[]>(DEFAULT_VOICES);
  const isPreviewing = useSignal(false);
  const previewError = useSignal<string | null>(null);
  const isRecordingMic = useSignal(false);
  const importedFileName = useSignal<string | null>(null);

  useSignalEffect(() => {
    invoke<VoicePersonaInfo[]>('get_available_tts_voices')
      .then((voices) => {
        if (voices && voices.length > 0) {
          availableVoices.value = voices;
        }
      })
      .catch(() => {
        // Fallback to default presets
      });
  });

  const handleTestPreview = async () => {
    if (isPreviewing.value) return;
    previewError.value = null;
    isPreviewing.value = true;

    try {
      if (soundMode === 'default') {
        await invoke('test_voice_macro_sound');
      } else if (soundMode === 'tts') {
        const text = ttsText.trim() || 'Command confirmed';
        const voice = ttsVoice || 'piper-en_US-amy-low';
        await invoke('preview_tts_voice', {
          text,
          voiceId: voice,
          speed: ttsSpeed || 1.0,
          effect: ttsEffect || 'clean',
          pitch: ttsPitch || 0.0,
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
              <label
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  color: tokens.colors.textMuted,
                  textTransform: 'uppercase',
                }}
              >
                AI Voice Persona
              </label>
              <SelectField
                value={ttsVoice || 'piper-en_US-amy-low'}
                options={availableVoices.value.map((v) => ({
                  value: v.id,
                  label: `${v.name} — ${v.persona}`,
                  searchText: `${v.name} ${v.persona} ${v.category} ${v.description}`,
                }))}
                onChange={(val) => onTtsVoiceChange(val)}
                ariaLabel="TTS Voice Persona"
                style={{ width: '100%', fontSize: '12px' }}
              />
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              <label
                style={{
                  fontSize: '11px',
                  fontWeight: 600,
                  color: tokens.colors.textMuted,
                  textTransform: 'uppercase',
                }}
              >
                Audio Effect Profile
              </label>
              <SelectField
                value={ttsEffect || 'clean'}
                options={EFFECT_OPTIONS}
                onChange={(val) => onTtsEffectChange?.(val)}
                ariaLabel="Audio Effect Profile"
                style={{ width: '100%', fontSize: '12px' }}
              />
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '8px' }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <label
                  style={{
                    fontSize: '11px',
                    fontWeight: 600,
                    color: tokens.colors.textMuted,
                    textTransform: 'uppercase',
                  }}
                >
                  Tone / Pitch Adjustment
                </label>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(6, 1fr)', gap: '3px' }}>
                  {PITCH_OPTIONS.map((opt) => {
                    const currentPitch = ttsPitch ?? 0;
                    const isSelected = Math.abs(currentPitch - opt.value) < 0.1;
                    return (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() => onTtsPitchChange?.(opt.value)}
                        style={{
                          padding: '4px 1px',
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
                        {opt.value > 0 ? `+${opt.value}` : opt.value}
                      </button>
                    );
                  })}
                </div>
              </div>

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
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '3px' }}>
                  {SPEED_OPTIONS.map((opt) => {
                    const currentSpeed = ttsSpeed || 1.0;
                    const isSelected = Math.abs(currentSpeed - opt.value) < 0.01;
                    return (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() => onTtsSpeedChange(opt.value)}
                        style={{
                          padding: '4px 1px',
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
    </div>
  );
}
