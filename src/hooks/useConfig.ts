import { useState, useEffect, useRef, useCallback } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import type { Config, ModelInfo } from '../types.ts';

interface UseConfigReturn {
  config: Config;
  setConfig: (config: Config) => void;
  lastCommittedConfigRef: { current: Config | null };
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
  isDownloading: boolean;
  loadConfig: () => Promise<void>;
  loadModels: () => Promise<void>;
  downloadModel: (size: string) => Promise<void>;
  persistConfig: (configToPersist: Config, showSavedConfirmation?: boolean) => Promise<void>;
  updateConfig: (key: string, value: string | number | boolean | null) => void;
  toggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
  formatConfigValueForLog: (key: keyof Config, value: Config[keyof Config]) => string;
  hasLoadedConfig: boolean;
  hasLoadedModels: boolean;
}

export function useConfig(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void, logUI: (msg: string) => void): UseConfigReturn {
  const [config, setConfig] = useState<Config>({
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
    enable_gpu: false,
    post_roll_ms: 400,
    hotkey_mode: 'HoldToTalk',
    max_recording_duration_minutes: 10,
  });
  const [availableEngines, setAvailableEngines] = useState<string[]>([]);
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([]);
  const [modelStatus, setModelStatus] = useState<Record<string, boolean>>({});
  const [downloadProgress, setDownloadProgress] = useState<number>(0);
  const [isDownloading, setIsDownloading] = useState(false);
  const [hasLoadedConfig, setHasLoadedConfig] = useState(false);
  const [hasLoadedModels, setHasLoadedModels] = useState(false);
  const lastCommittedConfigRef = useRef<Config | null>(null);

  const formatConfigValueForLog = useCallback((key: keyof Config, value: Config[keyof Config]) => {
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
  }, []);

  const loadConfig = useCallback(async () => {
    try {
      const savedConfig = await invoke<Config>('get_config');
      setConfig({
        ...savedConfig,
        typing_speed_interval: Math.round(savedConfig.typing_speed_interval * 1000),
      });
    } catch (error) {
      showToast(`Failed to load config: ${error}`, 'error');
    } finally {
      setHasLoadedConfig(true);
    }
  }, [showToast]);

  const loadModels = useCallback(async () => {
    try {
      const engines = await invoke<string[]>('get_available_engines');
      setAvailableEngines(engines || []);

      const models = await invoke<ModelInfo[]>('get_available_models');
      setAvailableModels(models || []);

      const status: Record<string, boolean> = {};
      for (const model of (models || [])) {
        status[model.size] = await invoke<boolean>('check_model_status', { modelSize: model.size });
      }
      setModelStatus(status);
    } catch (error) {
      showToast(`Failed to load models: ${error}`, 'error');
    } finally {
      setHasLoadedModels(true);
    }
  }, [showToast]);

  const downloadModel = useCallback(async (size: string) => {
    setIsDownloading(true);
    setDownloadProgress(0);
    try {
      await invoke('download_model', { modelSize: size });
      showToast(`${size} model downloaded successfully!`, 'success');
      await loadModels();
    } catch (error) {
      showToast(`Failed to download model: ${error}`, 'error');
    } finally {
      setIsDownloading(false);
      setDownloadProgress(0);
    }
  }, [showToast, loadModels]);

  const persistConfig = useCallback(async (configToPersist: Config, showSavedConfirmation = false) => {
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
  }, [showToast]);

  const updateConfig = useCallback((key: string, value: string | number | boolean | null) => {
    const normalizedValue = key === 'input_sensitivity'
      ? (() => {
          const parsedValue = Number(value);
          if (!Number.isFinite(parsedValue)) {
            return 1.0;
          }
          return Math.min(2.0, Math.max(0.1, parsedValue));
        })()
      : value;
    setConfig(prev => ({ ...prev, [key]: normalizedValue } as Config));
  }, []);

  const toggleOutputMethod = useCallback((method: 'Typewriter' | 'Clipboard') => {
    logUI('Output Method changed to: ' + method);
    updateConfig('output_method', method);
  }, [logUI, updateConfig]);

  // Auto-save config with 500ms debounce
  useEffect(() => {
    const timer = setTimeout(() => {
      const previousConfig = lastCommittedConfigRef.current;
      let hasChanges = false;
      if (previousConfig) {
        (Object.keys(config) as (keyof Config)[]).forEach((key) => {
          if (previousConfig[key] !== config[key]) {
            hasChanges = true;
            const formattedValue = formatConfigValueForLog(key, config[key]);
            logUI('Setting changed: ' + key + ' -> ' + formattedValue);
          }
        });
      }

      lastCommittedConfigRef.current = { ...config };
      if (previousConfig === null || hasChanges) {
        persistConfig(config, hasChanges && previousConfig !== null);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [config, persistConfig, formatConfigValueForLog, logUI]);

  // Validate model selection when engine changes
  useEffect(() => {
    if (availableModels.length > 0) {
      const modelsForEngine = availableModels.filter(m => m.engine === config.local_engine);
      const isCurrentModelValid = modelsForEngine.some(m => m.size === config.local_model_size);

      if (!isCurrentModelValid && modelsForEngine.length > 0) {
        const recommended = modelsForEngine.find(m => m.recommended) || modelsForEngine[0];
        updateConfig('local_model_size', recommended.size);
      }
    }
  }, [config.local_engine, availableModels, updateConfig]);

  // Auto-load models when switching to Local mode
  useEffect(() => {
    if (config.transcription_mode === 'Local' && availableModels.length === 0) {
      loadModels();
    }
  }, [config.transcription_mode, availableModels.length, loadModels]);

  return {
    config,
    setConfig,
    lastCommittedConfigRef,
    availableEngines,
    availableModels,
    modelStatus,
    downloadProgress,
    isDownloading,
    loadConfig,
    loadModels,
    downloadModel,
    persistConfig,
    updateConfig,
    toggleOutputMethod,
    formatConfigValueForLog,
    hasLoadedConfig,
    hasLoadedModels,
  };
}