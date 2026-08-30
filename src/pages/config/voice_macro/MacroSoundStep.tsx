import { useEffect } from 'preact/hooks';
import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
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
  IconAlertTriangle,
  IconAdjustmentsHorizontal,
} from '@tabler/icons-preact';
import type { DownloadPhase, MacroSoundMode, TtsModelDownloadProgress, VoicePreset } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { Button } from '../../../components/Button.tsx';
import { DownloadProgressBar } from '../../../components/DownloadProgressBar.tsx';
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
  ttsSpeed?: number;
  onTtsSpeedChange?: (speed: number) => void;
  ttsEffect?: string;
  onTtsEffectChange?: (effect: string) => void;
  ttsPitch?: number;
  onTtsPitchChange?: (pitch: number) => void;
}

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
  const customPresets = useSignal<VoicePreset[]>([]);
  const isPlaying = useSignal(false);
  const isSynthesizing = useSignal(false);
  const isDownloading = useSignal(false);
  const downloadProgress = useSignal(0);
  const downloadPhase = useSignal<DownloadPhase>('downloading');
  const playTimer = useSignal<ReturnType<typeof setTimeout> | null>(null);
  const previewError = useSignal<string | null>(null);
  const isRecordingMic = useSignal(false);
  const importedFileName = useSignal<string | null>(null);
  const isVoiceLabModalOpen = useSignal(false);

  const fetchPresets = () => {
    invoke<VoicePreset[]>('get_custom_voice_presets')
      .then((presets) => {
        const list = presets || [];
        customPresets.value = list;
        if (list.length > 0) {
          const currentFound = list.find((p) => p.id === ttsVoice);
          if (!currentFound && (!ttsVoice || ttsVoice === 'titan-mech' || ttsVoice === 'default')) {
            handleSelectPreset(list[0].id, list);
          }
        }
      })
      .catch((err) => {
        console.warn('Failed to fetch custom voice presets:', err);
        customPresets.value = [];
      });
  };

  useSignalEffect(() => {
    fetchPresets();
  });

  useEffect(() => {
    let isMounted = true;
    const unlistenPromise = listen<TtsModelDownloadProgress>(
      'tts-model-download-progress',
      (event) => {
        if (!isMounted) return;
        isDownloading.value = true;
        downloadProgress.value = event.payload.progress;
        downloadPhase.value = event.payload.phase;
      }
    );

    return () => {
      isMounted = false;
      unlistenPromise.then((fn) => fn());
    };
  }, []);

  const activePreset = customPresets.value.find((p) => p.id === ttsVoice) || customPresets.value[0];

  const handleSelectPreset = (selectedId: string, listOverride?: VoicePreset[]) => {
    onTtsVoiceChange(selectedId);
    const list = listOverride || customPresets.value;
    const matched = list.find((p) => p.id === selectedId);
    if (matched) {
      if (onTtsEffectChange) onTtsEffectChange('custom');
      if (matched.pitch !== undefined && matched.pitch !== null && onTtsPitchChange) {
        onTtsPitchChange(matched.pitch);
      }
      if (matched.speed !== undefined && matched.speed !== null && onTtsSpeedChange) {
        onTtsSpeedChange(matched.speed);
      }
    }
  };

  const handleTestPreview = async () => {
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

    if (isSynthesizing.value || isDownloading.value) return;
    previewError.value = null;
    isSynthesizing.value = true;

    try {
      if (soundMode === 'default') {
        await invoke('test_voice_macro_sound');
        isSynthesizing.value = false;
        isPlaying.value = true;
        if (playTimer.value) clearTimeout(playTimer.value);
        playTimer.value = setTimeout(() => { isPlaying.value = false; }, 800);
      } else if (soundMode === 'tts') {
        const text = ttsText.trim() || 'Command confirmed';
        const selectedId = ttsVoice || activePreset?.id;
        if (!selectedId) {
          previewError.value = 'Please create or select a custom voice preset in Voice Studio first.';
          isSynthesizing.value = false;
          return;
        }
        const res = await invoke<{ duration_secs: number }>('preview_tts_voice', {
          text,
          voiceId: selectedId,
          speed: ttsSpeed || activePreset?.speed || 1.0,
          effect: ttsEffect || 'custom',
          pitch: ttsPitch ?? activePreset?.pitch ?? 0.0,
        });
        isDownloading.value = false;
        isSynthesizing.value = false;
        isPlaying.value = true;
        fetchPresets();
        const durMs = Math.round(((res && res.duration_secs) || 3.0) * 1000) + 400;
        if (playTimer.value) clearTimeout(playTimer.value);
        playTimer.value = setTimeout(() => { isPlaying.value = false; }, durMs);
      } else {
        await invoke('play_macro_sound_preview', { macroId });
        isSynthesizing.value = false;
        isPlaying.value = true;
        if (playTimer.value) clearTimeout(playTimer.value);
        playTimer.value = setTimeout(() => { isPlaying.value = false; }, 2500);
      }
    } catch (e) {
      previewError.value = String(e);
      isDownloading.value = false;
      isSynthesizing.value = false;
      isPlaying.value = false;
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
    <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', width: '100%' }}>
      {/* Sound Mode Selection Pills */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
        <label
          style={{
            fontSize: '11px',
            fontWeight: 600,
            color: tokens.colors.textMuted,
            textTransform: 'uppercase',
            letterSpacing: '0.4px',
          }}
        >
          Audio Feedback Mode
        </label>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(5, 1fr)',
            gap: '4px',
            background: 'rgba(0, 0, 0, 0.25)',
            padding: '3px',
            borderRadius: '999px',
            border: '1px solid rgba(255, 255, 255, 0.08)',
          }}
        >
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
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '4px',
                  padding: '5px 4px',
                  borderRadius: '999px',
                  fontSize: '11px',
                  fontWeight: isActive ? 600 : 500,
                  background: isActive
                    ? 'linear-gradient(135deg, rgba(88, 101, 242, 0.35) 0%, rgba(129, 140, 248, 0.2) 100%)'
                    : 'transparent',
                  border: isActive
                    ? '1px solid rgba(99, 102, 241, 0.55)'
                    : '1px solid transparent',
                  color: isActive ? '#ffffff' : tokens.colors.textSecondary,
                  cursor: 'pointer',
                  transition: tokens.transitions.fast,
                  whiteSpace: 'nowrap',
                }}
              >
                <IconComponent size={13} color={isActive ? '#818cf8' : tokens.colors.textMuted} />
                <span style={{ fontSize: '10.5px' }}>{tab.label}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Mode Specific Settings */}
      {soundMode === 'tts' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
          {/* Spoken Confirmation Phrase */}
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
              style={{ ...inputBaseStyle, padding: '8px 10px', fontSize: '12px' }}
            />
          </div>

          {/* Voice Preset Picker */}
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
                Voice Preset
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
                <span>Voice Studio</span>
              </button>
            </div>

            {customPresets.value.length > 0 ? (
              <SelectField
                value={ttsVoice || customPresets.value[0]?.id}
                options={customPresets.value.map((p) => ({
                  value: p.id,
                  label: p.name,
                  searchText: `${p.name} ${p.description || ''} ${p.model_key || ''}`,
                }))}
                onChange={(val) => handleSelectPreset(val)}
                ariaLabel="Voice Preset"
                style={{ width: '100%', fontSize: '12px' }}
              />
            ) : (
              <div
                style={{
                  padding: '10px 12px',
                  borderRadius: '8px',
                  background: 'rgba(88, 101, 242, 0.08)',
                  border: '1px solid rgba(88, 101, 242, 0.25)',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '6px',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11.5px', color: tokens.colors.textPrimary, fontWeight: 600 }}>
                  <IconSparkles size={14} color="#9ba5ff" />
                  <span>No Custom Presets Yet</span>
                </div>
                <p style={{ fontSize: '11px', color: tokens.colors.textMuted, margin: 0, lineHeight: 1.4 }}>
                  Open Voice Studio to design, tune DSP filters, and save your custom voices.
                </p>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => { isVoiceLabModalOpen.value = true; }}
                  style={{ alignSelf: 'flex-start', marginTop: '2px', fontSize: '11px', padding: '4px 10px' }}
                >
                  <IconAdjustmentsHorizontal size={13} />
                  <span>Open Voice Studio</span>
                </Button>
              </div>
            )}
          </div>
        </div>
      )}

      {soundMode === 'custom_file' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <label
            style={{
              fontSize: '11px',
              fontWeight: 600,
              color: tokens.colors.textMuted,
              textTransform: 'uppercase',
            }}
          >
            Imported Audio Sound File
          </label>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Button
              variant="secondary"
              onClick={handleBrowseFile}
              style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}
            >
              <IconUpload size={14} />
              <span>Choose Audio File...</span>
            </Button>
            {importedFileName.value && (
              <span style={{ fontSize: '12px', color: tokens.colors.textSecondary }}>
                {importedFileName.value}
              </span>
            )}
          </div>
          <p style={{ fontSize: '11px', color: tokens.colors.textMuted, margin: 0 }}>
            Supports MP3, WAV, FLAC, OGG, M4A, AAC. Audio is normalized and cached with the macro.
          </p>
        </div>
      )}

      {soundMode === 'mic_recording' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <label
            style={{
              fontSize: '11px',
              fontWeight: 600,
              color: tokens.colors.textMuted,
              textTransform: 'uppercase',
            }}
          >
            Record Directly from Microphone
          </label>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Button
              variant={isRecordingMic.value ? 'danger' : 'secondary'}
              onClick={handleToggleMicRecord}
              style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}
            >
              <IconMicrophone size={14} />
              <span>{isRecordingMic.value ? 'Stop Recording & Save' : 'Record Mic Snippet'}</span>
            </Button>
            {isRecordingMic.value && (
              <span style={{ fontSize: '11px', color: '#ef4444', fontWeight: 600 }}>
                ● Recording in progress... speak now
              </span>
            )}
          </div>
        </div>
      )}

      {soundMode === 'default' && (
        <div
          style={{
            padding: '10px 12px',
            borderRadius: '8px',
            background: 'rgba(255, 255, 255, 0.03)',
            border: '1px solid rgba(255, 255, 255, 0.06)',
            fontSize: '11.5px',
            color: tokens.colors.textSecondary,
          }}
        >
          Plays the default Voquill tactical audio chime on macro execution.
        </div>
      )}

      {soundMode === 'none' && (
        <div
          style={{
            padding: '10px 12px',
            borderRadius: '8px',
            background: 'rgba(255, 255, 255, 0.03)',
            border: '1px solid rgba(255, 255, 255, 0.06)',
            fontSize: '11.5px',
            color: tokens.colors.textMuted,
          }}
        >
          Silent execution. No audio confirmation will be played.
        </div>
      )}

      {/* Model Download Progress Bar */}
      {isDownloading.value && (
        <div style={{ width: '100%', marginTop: '4px' }}>
          <DownloadProgressBar
            isDownloading={isDownloading.value}
            progress={downloadProgress.value}
            phase={downloadPhase.value}
            itemLabel={activePreset ? activePreset.name : 'TTS Voice Model'}
          />
        </div>
      )}

      {/* Test Preview Action Button */}
      {soundMode !== 'none' && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '4px' }}>
          <Button
            variant={isPlaying.value ? 'danger' : 'primary'}
            onClick={handleTestPreview}
            disabled={isSynthesizing.value || isDownloading.value}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '6px 14px',
              fontSize: '12px',
            }}
          >
            {isSynthesizing.value ? (
              <>
                <IconLoader2 size={14} className="spin" />
                <span>Synthesizing Voice...</span>
              </>
            ) : isDownloading.value ? (
              <>
                <IconLoader2 size={14} className="spin" />
                <span>Downloading Voice...</span>
              </>
            ) : isPlaying.value ? (
              <>
                <IconPlayerStop size={14} />
                <span>Stop Preview</span>
              </>
            ) : (
              <>
                <IconPlayerPlay size={14} />
                <span>Test Audio Preview</span>
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

      {isVoiceLabModalOpen.value && (
        <VoiceLabModal
          onClose={() => {
            isVoiceLabModalOpen.value = false;
          }}
          onPresetSaved={(newPreset) => {
            fetchPresets();
            handleSelectPreset(newPreset.id);
          }}
        />
      )}
    </div>
  );
}
