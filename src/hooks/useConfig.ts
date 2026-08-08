import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { Config, EngineCapabilities, ModelInfo } from '../types.ts';

interface UseConfigReturn {
  config: Config;
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  engineCapabilities: EngineCapabilities | null;
  downloadProgress: number;
  isDownloading: boolean;
  loadConfig: () => Promise<void>;
  loadModels: () => Promise<void>;
  downloadModel: (size: string) => Promise<void>;
  persistConfig: (configToPersist: Config, showSavedConfirmation?: boolean) => Promise<void>;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown>) => void;
  toggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
  formatConfigValueForLog: (key: keyof Config, value: Config[keyof Config]) => string;
  hasLoadedConfig: boolean;
  hasLoadedModels: boolean;
}

export function useConfig(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void, logUI: (msg: string) => void): UseConfigReturn {
  const config = useSignal<Config>({
    openai_api_key: '',
    api_url: 'https://api.openai.com/v1/audio/transcriptions',
    api_model: 'whisper-1',
    transcription_mode: 'Local',
    local_model_size: 'base',
    local_engine: 'Whisper.cpp',
    hotkey: 'ctrl+shift+space',
    typing_speed_interval: 1,
    key_press_duration_ms: 2,
    pixels_from_bottom: 100,
    audio_device: 'default',
    debug_mode: false,
    enable_recording_logs: false,
    input_sensitivity: 1.0,
    output_method: 'Typewriter',
    copy_on_typewriter: false,
    language: 'auto',
    post_roll_ms: 400,
    hotkey_mode: 'HoldToTalk',
    max_recording_duration_minutes: 10,
    engine_config: null,
    dictionary: [],
    post_process_enabled: false,
    post_process_provider: 'Local',
    post_process_model: 'qwen2.5-1.5b-instruct',
    post_process_api_url: '',
    post_process_api_key: '',
  });
  const availableEngines = useSignal<string[]>([]);
  const availableModels = useSignal<ModelInfo[]>([]);
  const modelStatus = useSignal<Record<string, boolean>>({});
  const engineCapabilities = useSignal<EngineCapabilities | null>(null);
  const downloadProgress = useSignal<number>(0);
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
      const savedConfig = await invoke<Config>('get_config');
      config.value = {
        ...savedConfig,
        typing_speed_interval: Math.round(savedConfig.typing_speed_interval * 1000),
      };
    } catch (error) {
      showToast(`Failed to load config: ${error}`, 'error');
    } finally {
      hasLoadedConfig.value = true;
    }
  };

  const loadModels = async () => {
    try {
      const engines = await invoke<string[]>('get_available_engines');
      availableEngines.value = engines || [];

      const models = await invoke<ModelInfo[]>('get_available_models');
      availableModels.value = models || [];

      const status: Record<string, boolean> = {};
      for (const model of (models || [])) {
        status[model.size] = await invoke<boolean>('check_model_status', { modelSize: model.size, engineName: model.engine });
      }
      modelStatus.value = status;
    } catch (error) {
      showToast(`Failed to load models: ${error}`, 'error');
    } finally {
      hasLoadedModels.value = true;
    }
  };

  const downloadModel = async (size: string) => {
    isDownloading.value = true;
    downloadProgress.value = 0;
    try {
      await invoke('download_model', { modelSize: size, engineName: config.value.local_engine });
      showToast(`${size} model downloaded successfully!`, 'success');
      await loadModels();
    } catch (error) {
      showToast(`Failed to download model: ${error}`, 'error');
    } finally {
      isDownloading.value = false;
      downloadProgress.value = 0;
    }
  };

  const persistConfig = async (configToPersist: Config, showSavedConfirmation = false) => {
    try {
      const configToSave = {
        ...configToPersist,
        typing_speed_interval: configToPersist.typing_speed_interval / 1000,
        openai_api_key: configToPersist.openai_api_key || 'your_api_key_here',
      };
      await invoke('save_config', { newConfig: configToSave });
      if (showSavedConfirmation) {
        showToast('Saved', 'saved');
      }
    } catch (error) {
      showToast(`Failed to save: ${error}`, 'error');
    }
  };

  const updateConfig = (key: string, value: string | number | boolean | null | string[] | Record<string, unknown>) => {
    const normalizedValue = key === 'input_sensitivity'
      ? (() => {
          const parsedValue = Number(value);
          if (!Number.isFinite(parsedValue)) {
            return 1.0;
          }
          return Math.min(2.0, Math.max(0.1, parsedValue));
        })()
      : value;
    config.value = { ...config.value, [key]: normalizedValue } as Config;
  };

  const toggleOutputMethod = (method: 'Typewriter' | 'Clipboard') => {
    logUI('Output Method changed to: ' + method);
    updateConfig('output_method', method);
  };

  // Auto-save config with 500ms debounce
  useSignalEffect(() => {
    const currentConfig = config.value;
    const timer = setTimeout(() => {
      const previousConfig = lastCommittedConfig.value;
      let hasChanges = false;
      if (previousConfig) {
        (Object.keys(currentConfig) as (keyof Config)[]).forEach((key) => {
          if (JSON.stringify(previousConfig[key]) !== JSON.stringify(currentConfig[key])) {
            hasChanges = true;
            const formattedValue = formatConfigValueForLog(key, currentConfig[key]);
            logUI('Setting changed: ' + key + ' -> ' + formattedValue);
          }
        });
      }

      lastCommittedConfig.value = { ...currentConfig };
      if (previousConfig === null || hasChanges) {
        persistConfig(currentConfig, hasChanges && previousConfig !== null);
      }
    }, 500);
    return () => clearTimeout(timer);
  });

  // Validate model selection when engine changes
  useSignalEffect(() => {
    const models = availableModels.value;
    const localEngine = config.value.local_engine;
    if (models.length > 0) {
      const modelsForEngine = models.filter(m => m.engine === localEngine);
      const isCurrentModelValid = modelsForEngine.some(m => m.size === config.value.local_model_size);

      if (!isCurrentModelValid && modelsForEngine.length > 0) {
        const recommended = modelsForEngine.find(m => m.recommended) || modelsForEngine[0];
        updateConfig('local_model_size', recommended.size);
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
    modelStatus: modelStatus.value,
    engineCapabilities: engineCapabilities.value,
    downloadProgress: downloadProgress.value,
    isDownloading: isDownloading.value,
    loadConfig,
    loadModels,
    downloadModel,
    persistConfig,
    updateConfig,
    toggleOutputMethod,
    formatConfigValueForLog,
    hasLoadedConfig: hasLoadedConfig.value,
    hasLoadedModels: hasLoadedModels.value,
  };
}