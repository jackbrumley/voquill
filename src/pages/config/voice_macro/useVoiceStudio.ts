import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { BaseVoiceModelInfo, DownloadPhase, TtsModelDownloadProgress, VoicePreset } from '../../../types.ts';
import {
  DEFAULT_VOICE_SETTINGS,
  RADIO_ON_DEFAULTS,
  presetFromSettings,
  previewParams,
  resetTuning,
  settingsFromPreset,
  type VoiceSettings,
} from './voiceStudioSettings.ts';

const DEFAULT_TEST_PHRASE = 'Titan online. Core temperature nominal. All weapon systems combat ready.';

/// Owns Voice Studio: the voice being tuned (one `VoiceSettings` value), its
/// preview playback, the voice model download, and the saved presets.
export function useVoiceStudio(onPresetSaved?: (preset: VoicePreset) => void) {
  const settings = useSignal<VoiceSettings>(DEFAULT_VOICE_SETTINGS);
  const testPhrase = useSignal(DEFAULT_TEST_PHRASE);
  const presetName = useSignal('');
  const currentPresetId = useSignal<string | null>(null);
  const models = useSignal<BaseVoiceModelInfo[]>([]);
  const savedPresets = useSignal<VoicePreset[]>([]);

  const isPlaying = useSignal(false);
  const isSynthesizing = useSignal(false);
  const isDownloading = useSignal(false);
  const downloadProgress = useSignal(0);
  const downloadPhase = useSignal<DownloadPhase>('downloading');
  const playTimer = useSignal<ReturnType<typeof setTimeout> | null>(null);
  const error = useSignal<string | null>(null);
  const saveMessage = useSignal<string | null>(null);

  const refreshModels = () => {
    invoke<BaseVoiceModelInfo[]>('get_available_base_voice_models')
      .then((reported) => {
        models.value = reported;
      })
      .catch((e) => {
        error.value = `Failed to list voices: ${e}`;
      });
  };

  useEffect(() => {
    refreshModels();
    invoke<VoicePreset[]>('get_custom_voice_presets')
      .then((presets) => {
        if (presets) savedPresets.value = presets;
      })
      .catch(() => {});

    let isMounted = true;
    const unlistenPromise = listen<TtsModelDownloadProgress>('tts-model-download-progress', (event) => {
      if (!isMounted) return;
      isDownloading.value = true;
      downloadProgress.value = event.payload.progress;
      downloadPhase.value = event.payload.phase;
    });
    return () => {
      isMounted = false;
      unlistenPromise.then((stop) => stop());
    };
  }, []);

  const update = (changes: Partial<VoiceSettings>) => {
    settings.value = { ...settings.value, ...changes };
  };

  /// Switching the radio effect on starts from audible drive and noise.
  const setRadioBandpass = (isOn: boolean) => {
    if (isOn && settings.value.radio_drive <= 1.01) {
      update({ radio_bandpass: true, ...RADIO_ON_DEFAULTS });
    } else if (isOn) {
      update({ radio_bandpass: true });
    } else {
      update({
        radio_bandpass: false,
        radio_drive: DEFAULT_VOICE_SETTINGS.radio_drive,
        rf_noise: DEFAULT_VOICE_SETTINGS.rf_noise,
      });
    }
  };

  const resetTuningKnobs = () => {
    settings.value = resetTuning(settings.value);
  };

  const startNewPreset = () => {
    currentPresetId.value = null;
    presetName.value = '';
    resetTuningKnobs();
  };

  const stopPreview = async () => {
    if (playTimer.value) clearTimeout(playTimer.value);
    isPlaying.value = false;
    try {
      await invoke('stop_macro_sound_playback');
    } catch (e) {
      console.warn('Failed to stop sound:', e);
    }
  };

  const togglePreview = async () => {
    if (isPlaying.value) {
      await stopPreview();
      return;
    }
    if (isSynthesizing.value || isDownloading.value) return;
    error.value = null;
    isSynthesizing.value = true;
    try {
      const result = await invoke<{ duration_secs: number }>('preview_custom_tts_voice', {
        params: previewParams(settings.value, testPhrase.value),
      });
      isDownloading.value = false;
      isSynthesizing.value = false;
      isPlaying.value = true;
      // A preview may have downloaded a new voice model.
      refreshModels();
      const durationMs = Math.round(((result && result.duration_secs) || 3.0) * 1000) + 400;
      if (playTimer.value) clearTimeout(playTimer.value);
      playTimer.value = setTimeout(() => {
        isPlaying.value = false;
      }, durationMs);
    } catch (e) {
      console.error('Preview custom voice failed:', e);
      error.value = String(e);
      isDownloading.value = false;
      isSynthesizing.value = false;
      isPlaying.value = false;
    }
  };

  const savePreset = async () => {
    const name = presetName.value.trim();
    if (!name) return;
    const id = currentPresetId.value || `preset-${Date.now()}`;
    const preset = presetFromSettings(settings.value, id, name);
    try {
      const updatedMacroCount = await invoke<number>('save_custom_voice_preset', { preset });
      currentPresetId.value = id;
      savedPresets.value = [...savedPresets.value.filter((p) => p.id !== id), preset];
      saveMessage.value = updatedMacroCount > 0
        ? `Saved "${name}" and updated ${updatedMacroCount} macro voice${updatedMacroCount === 1 ? '' : 's'}!`
        : `Saved "${name}"!`;
      onPresetSaved?.(preset);
      setTimeout(() => {
        saveMessage.value = null;
      }, 3500);
    } catch (e) {
      error.value = `Failed to save preset: ${e}`;
    }
  };

  const loadPreset = (preset: VoicePreset) => {
    currentPresetId.value = preset.id;
    presetName.value = preset.name;
    settings.value = settingsFromPreset(preset);
  };

  const deletePreset = async (presetId: string) => {
    try {
      await invoke('delete_custom_voice_preset', { presetId });
      savedPresets.value = savedPresets.value.filter((p) => p.id !== presetId);
      if (currentPresetId.value === presetId) {
        currentPresetId.value = null;
        presetName.value = '';
      }
    } catch (e) {
      error.value = `Failed to delete preset: ${e}`;
    }
  };

  const selectedModel = models.value.find((model) => model.id === settings.value.model_key);

  return {
    settings: settings.value,
    testPhrase: testPhrase.value,
    presetName: presetName.value,
    currentPresetId: currentPresetId.value,
    models: models.value,
    isMultiSpeaker: selectedModel?.is_multi_speaker ?? false,
    savedPresets: savedPresets.value,
    isPlaying: isPlaying.value,
    isSynthesizing: isSynthesizing.value,
    isDownloading: isDownloading.value,
    downloadProgress: downloadProgress.value,
    downloadPhase: downloadPhase.value,
    error: error.value,
    saveMessage: saveMessage.value,
    update,
    setRadioBandpass,
    setTestPhrase: (phrase: string) => { testPhrase.value = phrase; },
    setPresetName: (name: string) => { presetName.value = name; },
    resetTuningKnobs,
    startNewPreset,
    togglePreview,
    savePreset,
    loadPreset,
    deletePreset,
  };
}

export type VoiceStudio = ReturnType<typeof useVoiceStudio>;
