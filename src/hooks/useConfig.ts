import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { backendDefaultConfig } from '../defaultConfig.ts';
import type { Config, DownloadPhase, EngineCapabilities, EngineInfo, ModelDownloadProgress, ModelInfo } from '../types.ts';

interface UseConfigReturn {
  config: Config;
  availableEngines: EngineInfo[];
  availableModels: ModelInfo[];
  engineCapabilities: EngineCapabilities | null;
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  isDownloading: boolean;
  loadConfig: () => Promise<void>;
  loadModels: () => Promise<void>;
  downloadModel: (size: string, engine?: string) => Promise<void>;
  setDownloadProgress: (val: ModelDownloadProgress) => void;
  /// Resolves to whether the backend accepted the config.
  persistConfig: (configToPersist: Config, showSavedConfirmation?: boolean) => Promise<boolean>;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
  toggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
  formatConfigValueForLog: (key: keyof Config, value: Config[keyof Config]) => string;
  hasLoadedConfig: boolean;
  hasLoadedModels: boolean;
}

/// Device selections take effect the moment they are picked, so a mic test or
/// dictation started right after the change already uses the new device.
const IMMEDIATE_SAVE_KEYS: ReadonlySet<keyof Config> = new Set<keyof Config>(['audio_device', 'playback_device']);

/// Debounce for every other setting, so typing or dragging a slider does not
/// save on every keystroke or tick.
const DEBOUNCED_SAVE_DELAY_MS = 500;

export function useConfig(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void, logUI: (msg: string) => void): UseConfigReturn {
  // Placeholder until the saved settings load (never persisted); the
  // backend's defaults are the only source of default values.
  const config = useSignal<Config>(backendDefaultConfig());
  const availableEngines = useSignal<EngineInfo[]>([]);
  const availableModels = useSignal<ModelInfo[]>([]);
  const engineCapabilities = useSignal<EngineCapabilities | null>(null);
  const downloadProgress = useSignal<number>(0);
  const downloadPhase = useSignal<DownloadPhase>('downloading');
  const isDownloading = useSignal(false);
  const hasLoadedConfig = useSignal(false);
  const hasLoadedModels = useSignal(false);
  const lastCommittedConfig = useSignal<Config | null>(null);

  const formatConfigValueForLog = (key: keyof Config, value: Config[keyof Config]) => {
    if (key === 'openai_api_key') {
      const length = typeof value === 'string' ? value.length : 0;
      return length > 0 ? `[redacted:${length} chars]` : '[empty]';
    }
    if (key === 'shortcuts_token' || key === 'input_token') {
      return '[redacted-token]';
    }
    if (value === null || value === undefined) {
      return 'null';
    }
    if (typeof value === 'string') {
      return value;
    }
    return String(value);
  };

  const loadConfig = async () => {
    try {
      const loadedConfig = await invoke<Config>('get_config');
      // The backend already holds this config, so it becomes the committed
      // baseline: reloading must never trigger a save on its own.
      lastCommittedConfig.value = loadedConfig;
      config.value = loadedConfig;
      hasLoadedConfig.value = true;
    } catch (error) {
      showToast(`Failed to load config: ${error}`, 'error');
    }
  };

  const loadModels = async () => {
    try {
      availableEngines.value = (await invoke<EngineInfo[]>('get_available_engines')) || [];
      availableModels.value = (await invoke<ModelInfo[]>('get_available_models')) || [];
    } catch (error) {
      showToast(`Failed to load models: ${error}`, 'error');
    } finally {
      hasLoadedModels.value = true;
    }
  };

  const downloadModel = async (size: string, engine?: string) => {
    isDownloading.value = true;
    downloadProgress.value = 0;
    downloadPhase.value = 'downloading';
    try {
      const engineName = engine || config.value.local_engine;
      await invoke('download_model', { modelSize: size, engineName });
      showToast(`${size} model downloaded successfully!`, 'success');
      await loadModels();
    } catch (error) {
      showToast(`Failed to download model: ${error}`, 'error');
    } finally {
      isDownloading.value = false;
      downloadProgress.value = 0;
      downloadPhase.value = 'downloading';
    }
  };

  const setDownloadProgress = (val: ModelDownloadProgress) => {
    downloadPhase.value = val.phase;
    downloadProgress.value = val.progress;
  };

  const persistConfig = async (configToPersist: Config, showSavedConfirmation = false) => {
    try {
      const configToSave = {
        ...configToPersist,
        openai_api_key: configToPersist.openai_api_key || 'your_api_key_here',
      };
      await invoke('save_config', { newConfig: configToSave });
      if (showSavedConfirmation) {
        showToast('Saved', 'saved');
      }
      return true;
    } catch (error) {
      showToast(`Failed to save: ${error}`, 'error');
      return false;
    }
  };

  const validModelSizeForEngine = (engine: string, fallback: string): string => {
    const modelsForEngine = availableModels.value.filter((model) => model.engine === engine);
    if (modelsForEngine.length === 0) {
      return fallback;
    }
    const valid = modelsForEngine.find((model) => model.size === fallback);
    if (valid) {
      return fallback;
    }
    return (modelsForEngine.find((model) => model.recommended) || modelsForEngine[0]).size;
  };

  const updateConfig = (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => {
    // The backend clamps input sensitivity when it saves (Config::normalize).
    const normalizedValue = key === 'diarization_cluster_threshold'
      ? (() => {
          const parsedValue = Number(value);
          if (!Number.isFinite(parsedValue)) {
            return 0.7;
          }
          return Math.min(0.95, Math.max(0.3, parsedValue));
        })()
      : value;
    const nextConfig = { ...config.value, [key]: normalizedValue } as Config;
    if (key === 'local_engine') {
      nextConfig.local_model_size = validModelSizeForEngine(
        nextConfig.local_engine,
        nextConfig.local_model_size,
      );
    }
    config.value = nextConfig;
  };

  const toggleOutputMethod = (method: 'Typewriter' | 'Clipboard') => {
    logUI('Output Method changed to: ' + method);
    updateConfig('output_method', method);
  };

  const commitConfig = async (configToCommit: Config, changedKeys: (keyof Config)[]) => {
    changedKeys.forEach((key) => {
      logUI('Setting changed: ' + key + ' -> ' + formatConfigValueForLog(key, configToCommit[key]));
    });
    lastCommittedConfig.value = { ...configToCommit };
    const saved = await persistConfig(configToCommit, true);
    if (!saved) {
      // The backend rejected the change and still runs its previous config;
      // reload it so the UI never shows a selection that is not in effect.
      await loadConfig();
    }
  };

  // Auto-save config changes. Gated on the on-disk config having loaded:
  // until then `config` holds the in-memory defaults above, and persisting
  // them would overwrite the user's saved settings. Typed and dragged values
  // are debounced; device selections save at once.
  useSignalEffect(() => {
    const currentConfig = config.value;
    if (!hasLoadedConfig.value) {
      return;
    }
    const previousConfig = lastCommittedConfig.peek();
    if (!previousConfig) {
      return;
    }
    const changedKeys = (Object.keys(currentConfig) as (keyof Config)[]).filter(
      (key) => JSON.stringify(previousConfig[key]) !== JSON.stringify(currentConfig[key]),
    );
    if (changedKeys.length === 0) {
      return;
    }
    const saveDelayMs = changedKeys.some((key) => IMMEDIATE_SAVE_KEYS.has(key)) ? 0 : DEBOUNCED_SAVE_DELAY_MS;
    const timer = setTimeout(() => {
      void commitConfig(currentConfig, changedKeys);
    }, saveDelayMs);
    return () => clearTimeout(timer);
  });

  // Validate model selection when engine changes or models load. The
  // synchronous path in updateConfig already keeps engine+model consistent on
  // user edits; this effect is a safety net for pairings loaded from disk.
  useSignalEffect(() => {
    const models = availableModels.value;
    const localEngine = config.value.local_engine;
    if (hasLoadedConfig.value && models.length > 0) {
      const modelsForEngine = models.filter((model) => model.engine === localEngine);
      if (modelsForEngine.length > 0) {
        const corrected = validModelSizeForEngine(localEngine, config.value.local_model_size);
        if (corrected !== config.value.local_model_size) {
          updateConfig('local_model_size', corrected);
        }
      }
    }
  });

  // Auto-load models when switching to Local mode
  useSignalEffect(() => {
    if (config.value.transcription_mode === 'Local' && availableModels.value.length === 0) {
      loadModels();
    }
  });

  // Load engine capabilities when the engine changes
  useSignalEffect(() => {
    const engine = config.value.local_engine;
    if (config.value.transcription_mode === 'Local' && engine) {
      invoke<EngineCapabilities>('get_engine_capabilities', { engineName: engine })
        .then((caps) => { engineCapabilities.value = caps; })
        .catch(() => { engineCapabilities.value = null; });
    } else {
      engineCapabilities.value = null;
    }
  });

  return {
    config: config.value,
    availableEngines: availableEngines.value,
    availableModels: availableModels.value,
    engineCapabilities: engineCapabilities.value,
    downloadProgress: downloadProgress.value,
    downloadPhase: downloadPhase.value,
    isDownloading: isDownloading.value,
    loadConfig,
    loadModels,
    downloadModel,
    setDownloadProgress,
    persistConfig,
    updateConfig,
    toggleOutputMethod,
    formatConfigValueForLog,
    hasLoadedConfig: hasLoadedConfig.value,
    hasLoadedModels: hasLoadedModels.value,
  };
}