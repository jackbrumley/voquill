
import { useState, useEffect, useRef } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart';
import { open } from '@tauri-apps/plugin-shell';
import { TitleBar } from './components/TitleBar.tsx';
import { Modals } from './components/Modals.tsx';
import { MainLayout } from './components/MainLayout.tsx';
import { InitialSetupPage } from './pages/InitialSetupPage.tsx';
import { appShellStyle, appContentStyle } from './theme/ui-primitives.ts';
import { useToast } from './hooks/useToast.tsx';
import { useTauriEvents } from './hooks/useTauriEvents.ts';
import type {
  Config,
  HistoryItem,
  AudioDevice,
  LinuxPermissions,
  ConfigureHotkeyResult,
  HotkeyBindingState,
  SystemShortcutContext,
  OverlayPositioningCapabilities,
  ModelInfo,
  UpdateCheckResult,
  AppRoute,
} from './types.ts';

const DEFAULT_ROUTE: AppRoute = 'status';

const routeFromHash = (hash: string): AppRoute => {
  const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
  if (normalized === 'setup' || normalized === 'status' || normalized === 'history' || normalized === 'settings' || normalized === 'ui-lab') {
    return normalized;
  }
  return DEFAULT_ROUTE;
};

const hashHasExplicitRoute = (hash: string): boolean => {
  const normalized = hash.replace(/^#\/?/, '').trim().toLowerCase();
  return normalized.length > 0;
};

function App() {
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
  
  const [activeRoute, setActiveRoute] = useState<AppRoute>(routeFromHash(window.location.hash));
  const [isTestingApi, setIsTestingApi] = useState(false);
  const { showToast, ToastContainer } = useToast();
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [availableMics, setAvailableMics] = useState<AudioDevice[]>([]);
  const [micTestStatus, setMicTestStatus] = useState<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const [micVolume, setMicVolume] = useState<number>(0);
  const [micTestPassed, setMicTestPassed] = useState(false);
  const [activeConfigSection, setActiveConfigSection] = useState<string | null>(null);
  const [appVersion, setAppVersion] = useState<string>('');
  const [availableEngines, setAvailableEngines] = useState<string[]>([]);
  const [availableModels, setAvailableModels] = useState<ModelInfo[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<number>(0);
  const [isDownloading, setIsDownloading] = useState(false);
  const [modelStatus, setModelStatus] = useState<Record<string, boolean>>({});
  const [permissions, setPermissions] = useState<LinuxPermissions | null>(null);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [recordedKeys, setRecordedKeys] = useState<Set<string>>(new Set());
  const [showModelGuide, setShowModelGuide] = useState(false);
  const [portalVersion, setPortalVersion] = useState<number>(0);
  const [hotkeyBindingState, setHotkeyBindingState] = useState<HotkeyBindingState | null>(null);
  const [systemShortcutContext, setSystemShortcutContext] = useState<SystemShortcutContext | null>(null);
  const [overlayPositioningCapabilities, setOverlayPositioningCapabilities] = useState<OverlayPositioningCapabilities>({
    manual_offset_supported: false,
    detail: 'Manual overlay position adjustment is not available on your system.',
  });
  const [showHotkeyCaptureModal, setShowHotkeyCaptureModal] = useState(false);
  const [showSystemShortcutModal, setShowSystemShortcutModal] = useState(false);
  const [showFactoryResetModal, setShowFactoryResetModal] = useState(false);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [isApplyingHotkey, setIsApplyingHotkey] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [lastCheckedAt, setLastCheckedAt] = useState<number | null>(null);
  const [initialRouteChecked, setInitialRouteChecked] = useState(false);
  const [hasLoadedConfig, setHasLoadedConfig] = useState(false);
  const [hasLoadedSetupStatus, setHasLoadedSetupStatus] = useState(false);
  const [hasLoadedMics, setHasLoadedMics] = useState(false);
  const [hasLoadedModels, setHasLoadedModels] = useState(false);
  const [setupTouched, setSetupTouched] = useState(false);
  const [hoveredTopTab, setHoveredTopTab] = useState<AppRoute | null>(null);
  const tabContentRef = useRef<HTMLDivElement | null>(null);
  const trayFallbackNotifiedRef = useRef(false);

  useEffect(() => {
    invoke<number>('get_wayland_portal_version')
      .then(setPortalVersion)
      .catch(e => console.log("Not running Wayland portal version check:", e));

    invoke<HotkeyBindingState>('get_hotkey_binding_state')
      .then(setHotkeyBindingState)
      .catch(e => console.log('Hotkey binding state unavailable:', e));

    invoke<SystemShortcutContext>('get_system_shortcut_context')
      .then(setSystemShortcutContext)
      .catch(e => console.log('System shortcut context unavailable:', e));

    invoke<OverlayPositioningCapabilities>('get_overlay_positioning_capabilities')
      .then(setOverlayPositioningCapabilities)
      .catch(e => {
        setOverlayPositioningCapabilities({
          manual_offset_supported: false,
          detail: 'Manual overlay position adjustment is not available on your system.',
        });
        console.log('Overlay positioning capabilities unavailable:', e);
      });
  }, []);

  const navigate = (route: AppRoute, replace = false) => {
    const nextHash = `#/${route}`;
    if (window.location.hash === nextHash) {
      setActiveRoute(route);
      return;
    }

    if (replace) {
      window.history.replaceState(null, '', nextHash);
      setActiveRoute(route);
      return;
    }

    window.location.hash = nextHash;
  };

  const logUI = (msg: string) => {
    // Log key interaction traces always; drop other spam unless debug mode
    if (
      !config.debug_mode &&
      !msg.includes('Button clicked') &&
      !msg.includes('Toast') &&
      !msg.includes('Setting changed') &&
      !msg.includes('Switch toggled')
    ) {
      return;
    }
    const timestamp = new Date().toLocaleTimeString();
    console.log(`[${timestamp}] ${msg}`);
    invoke('log_ui_event', { message: msg }).catch((err) => {
      console.error(`Failed to send log to backend: ${err}`);
    });
  };

  const lastCommittedConfigRef = useRef<Config | null>(null);

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

  // Initialize app data once on mount
  useEffect(() => {
    loadConfig();
    loadMics();
    loadHistory();
    loadModels();
    checkSetupStatus();
    
    getVersion().then(setAppVersion).catch(err => console.error("Failed to get version:", err));
    void checkForUpdates(false);
    isAutostartEnabled()
      .then(setAutostartEnabled)
      .catch((error: unknown) => {
        console.log('Autostart state unavailable:', error);
      });
  }, []);

  // Handle hotkey recording separately
  useEffect(() => {
    if (!isRecordingHotkey) return;

    window.addEventListener('keydown', handleHotkeyKeyDown);
    window.addEventListener('keyup', handleHotkeyKeyUp);

    return () => {
      window.removeEventListener('keydown', handleHotkeyKeyDown);
      window.removeEventListener('keyup', handleHotkeyKeyUp);
    };
  }, [isRecordingHotkey, recordedKeys]);

  useEffect(() => {
    if (config.transcription_mode === 'Local' && availableModels.length === 0) {
      loadModels();
    }
  }, [config.transcription_mode]);

  useEffect(() => {
    if (tabContentRef.current) {
      tabContentRef.current.scrollTop = 0;
    }
  }, [activeRoute]);

  const checkSetupStatus = async () => {
    try {
      const perms = await invoke<LinuxPermissions>('get_linux_setup_status');
      setPermissions(perms);
      if (typeof perms.manual_overlay_offset_supported === 'boolean') {
        setOverlayPositioningCapabilities({
          manual_offset_supported: perms.manual_overlay_offset_supported,
          detail: perms.overlay_positioning_detail,
        });
      }
      const bindingState = await invoke<HotkeyBindingState>('get_hotkey_binding_state');
      setHotkeyBindingState(bindingState);
    } catch (error) {
      console.error('Failed to check setup status:', error);
    } finally {
      setHasLoadedSetupStatus(true);
    }
  };

  const handleAudioSetup = async () => {
    setSetupTouched(true);
    try {
      await invoke('request_audio_permission');
      showToast('Audio permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get audio permission: ${error}`, 'error');
    }
  };

  const handleInputSetup = async () => {
    setSetupTouched(true);
    try {
      await invoke('request_input_permission');
      showToast('Input permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get input permission: ${error}`, 'error');
    }
  };

  const handleConfigureHotkey = async () => {
    if (isApplyingHotkey) return;
    setSetupTouched(true);

    try {
      setIsApplyingHotkey(true);
      const result = await invoke<ConfigureHotkeyResult>('configure_hotkey');

      if (result.outcome === 'requires_in_app_capture') {
        setShowHotkeyCaptureModal(true);
        await setRecordingState(true);
        setRecordedKeys(new Set());
        showToast('Press your desired key combination in the modal.', 'info');
      } else if (result.outcome === 'system_managed') {
        setShowSystemShortcutModal(true);
      } else {
        showToast(result.detail || 'Shortcut configured successfully!', 'success');
        await checkSetupStatus();
      }
    } catch (error) {
      showToast(`Failed to configure shortcut: ${error}`, 'error');
    } finally {
      setIsApplyingHotkey(false);
    }
  };

  const applyCapturedHotkey = async (capturedHotkey: string) => {
    try {
      setIsApplyingHotkey(true);
      updateConfig('hotkey', capturedHotkey);
      await invoke<ConfigureHotkeyResult>('apply_captured_hotkey', { newHotkey: capturedHotkey });
      showToast('Shortcut configured successfully!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to apply captured shortcut: ${error}`, 'error');
    } finally {
      await setRecordingState(false);
      setRecordedKeys(new Set());
      setShowHotkeyCaptureModal(false);
      setIsApplyingHotkey(false);
    }
  };

  const loadConfig = async () => {
    try {
      const savedConfig = await invoke<Config>('get_config');
      setConfig({
        ...savedConfig,
        typing_speed_interval: Math.round(savedConfig.typing_speed_interval * 1000)
      });
    } catch (error) {
      showToast(`Failed to load config: ${error}`, 'error');
    } finally {
      setHasLoadedConfig(true);
    }
  };

  const loadMics = async () => {
    try {
      const devices = await invoke<AudioDevice[]>('get_audio_devices');
      setAvailableMics(devices);
    } catch (error) {
      showToast(`Failed to load microphones: ${error}`, 'error');
    } finally {
      setHasLoadedMics(true);
    }
  };

  const loadHistory = async () => {
    try {
      const savedHistory = await invoke<{ items: HistoryItem[] }>('get_history');
      setHistory(savedHistory.items || []);
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  };

  const loadModels = async () => {
    console.log('📡 Fetching available models...');
    try {
      const engines = await invoke<string[]>('get_available_engines');
      setAvailableEngines(engines || []);

      const models = await invoke<ModelInfo[]>('get_available_models');
      console.log('✅ Models received:', models);
      if (!models || models.length === 0) {
        console.warn('⚠️ No models returned from backend.');
      }
      setAvailableModels(models || []);
      
      const status: Record<string, boolean> = {};
      for (const model of (models || [])) {
        status[model.size] = await invoke<boolean>('check_model_status', { modelSize: model.size });
      }
      setModelStatus(status);
    } catch (error) {
      console.error('❌ Failed to load models:', error);
      showToast(`Failed to load models: ${error}`, 'error');
    } finally {
      setHasLoadedModels(true);
    }
  };

  const downloadModel = async (size: string) => {
    setSetupTouched(true);
    setIsDownloading(true);
    setDownloadProgress(0);
    try {
      await invoke('download_model', { modelSize: size });
      showToast(`${size} model downloaded successfully!`, 'success');
      loadModels();
    } catch (error) {
      showToast(`Failed to download model: ${error}`, 'error');
    } finally {
      setIsDownloading(false);
      setDownloadProgress(0);
    }
  };

  const clearHistory = async () => {
    try {
      await invoke('clear_history');
      setHistory([]);
      showToast('History cleared', 'success');
    } catch {
      showToast('Failed to clear history', 'error');
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await invoke('plugin:clipboard-manager|write_text', { text });
      showToast('Copied to clipboard', 'success');
    } catch {
      showToast('Failed to copy', 'error');
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
        showToast('✓ Saved', 'saved');
      }
    } catch (error) {
      console.error('Failed to auto-save configuration:', error);
      showToast(`Failed to save: ${error}`, 'error');
    }
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      const previousConfig = lastCommittedConfigRef.current;
      let hasChanges = false;
      if (previousConfig) {
        (Object.keys(config) as (keyof Config)[]).forEach((key) => {
          if (previousConfig[key] !== config[key]) {
            hasChanges = true;
            const formattedValue = formatConfigValueForLog(key, config[key]);
            logUI(`⚙️ Setting changed: ${key} -> ${formattedValue}`);
          }
        });
      }

      lastCommittedConfigRef.current = { ...config };
      // Only persist on actual changes (or the first run) — save_config emits
      // config-updated, which re-loads config; persisting unconditionally
      // would create a save loop.
      if (previousConfig === null || hasChanges) {
        persistConfig(config, hasChanges && previousConfig !== null);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [config]);

  useEffect(() => {
    if (availableModels.length > 0) {
      const modelsForEngine = availableModels.filter(m => m.engine === config.local_engine);
      const isCurrentModelValid = modelsForEngine.some(m => m.size === config.local_model_size);
      
      if (!isCurrentModelValid && modelsForEngine.length > 0) {
        // Find recommended or first model for this engine
        const recommended = modelsForEngine.find(m => m.recommended) || modelsForEngine[0];
        updateConfig('local_model_size', recommended.size);
      }
    }
  }, [config.local_engine, availableModels]);

  const updateConfig = (key: string, value: string | number | boolean | null) => {
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
  };

  const toggleOutputMethod = (method: 'Typewriter' | 'Clipboard') => {
    logUI(`🖱️ Output Method changed to: ${method}`);
    updateConfig('output_method', method);
  };

  const startMicTest = async () => {
    try {
      setMicTestStatus('recording');
      await invoke('start_mic_test');
    } catch (error) {
      setMicTestStatus('idle');
      showToast(`Failed to start mic test: ${error}`, 'error');
    }
  };

  const stopMicTest = async () => {
    setMicTestStatus('processing');
    try {
      await invoke('stop_mic_test');
    } catch (error) {
      setMicTestStatus('idle');
      showToast(`Failed to stop mic test: ${error}`, 'error');
    }
  };

  const stopMicPlayback = async () => {
    try {
      await invoke('stop_mic_playback');
      setMicTestStatus('idle');
    } catch (error) {
      showToast(`Failed to stop playback: ${error}`, 'error');
    }
  };

  const isLocalModelReady = config.transcription_mode !== 'Local' || !!modelStatus[config.local_model_size];
  const isAudioDeviceReady = availableMics.length > 0 && !!config.audio_device;
  const isPortalSetupReady =
    !!permissions && permissions.audio && permissions.shortcuts && permissions.input_emulation;
  const isSystemManagedShortcut = portalVersion >= 1;

  const openDebugFolder = async () => {
    try {
      await invoke('open_debug_folder');
    } catch {
      showToast('Failed to open debug folder', 'error');
    }
  };

  const openLatestReleasePage = async () => {
    const releaseUrl = updateResult?.releaseUrl || 'https://github.com/jackbrumley/voquill/releases/latest';
    try {
      await open(releaseUrl);
    } catch (error) {
      showToast(`Failed to open release page: ${error}`, 'error');
    }
  };

  const checkForUpdates = async (showUpToDateModal: boolean) => {
    if (checkingUpdates) {
      return;
    }

    setCheckingUpdates(true);
    try {
      const result = await invoke<UpdateCheckResult>('check_for_updates');
      setUpdateResult(result);
      setLastCheckedAt(Date.now());
      if (result.updateAvailable || showUpToDateModal) {
        setShowUpdateModal(true);
      }
      if (!result.updateAvailable && showUpToDateModal) {
        showToast('You are already on the latest version.', 'info');
      }
    } catch (error) {
      if (showUpToDateModal) {
        showToast(`Failed to check for updates: ${error}`, 'error');
      } else {
        console.log('Background update check failed:', error);
      }
    } finally {
      setCheckingUpdates(false);
    }
  };

  const toggleAutostart = async (enabled: boolean) => {
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      setAutostartEnabled(enabled);
      showToast(`Auto-start ${enabled ? 'enabled' : 'disabled'}`, 'success');
    } catch (error) {
      showToast(`Failed to toggle auto-start: ${error}`, 'error');
    }
  };

  const getLastCheckedLabel = () => {
    if (!lastCheckedAt) {
      return 'Not checked yet';
    }

    const elapsedMs = Date.now() - lastCheckedAt;
    if (elapsedMs < 60_000) {
      return 'Just now';
    }

    const elapsedMinutes = Math.floor(elapsedMs / 60_000);
    if (elapsedMinutes < 60) {
      return `${elapsedMinutes} min ago`;
    }

    const elapsedHours = Math.floor(elapsedMinutes / 60);
    if (elapsedHours < 24) {
      return `${elapsedHours} hr ago`;
    }

    const elapsedDays = Math.floor(elapsedHours / 24);
    return `${elapsedDays} day${elapsedDays === 1 ? '' : 's'} ago`;
  };

  const testApiKey = async () => {
    setIsTestingApi(true);
    try {
      const isValid = await invoke<boolean>('test_api_key', { 
        apiKey: config.openai_api_key,
        apiUrl: config.api_url 
      });
      if (isValid) {
        showToast('API Key is valid!', 'success');
      } else {
        showToast('API Key is invalid or rate limited.', 'error');
      }
    } catch (error) {
      showToast(`API Test Failed: ${error}`, 'error');
    } finally {
      setIsTestingApi(false);
    }
  };

  const handleFactoryReset = async () => {
    try {
      await invoke('reset_application_to_defaults');
      setShowFactoryResetModal(false);
      showToast('Factory reset completed.', 'success');

      await Promise.all([
        loadConfig(),
        loadMics(),
        loadModels(),
        loadHistory(),
        checkSetupStatus(),
      ]);

      setSetupTouched(false);
      setInitialRouteChecked(false);
      navigate('setup', true);
    } catch (error) {
      showToast(`Factory reset failed: ${error}`, 'error');
    }
  };

  const handleClose = async () => {
    try {
      await invoke('quit_application');
    } catch {
      await getCurrentWindow().close();
    }
  };

  const handleMinimize = async () => {
    try {
      const target = await invoke<string>('minimize_to_tray_or_taskbar');
      if (target === 'taskbar' && !trayFallbackNotifiedRef.current) {
        trayFallbackNotifiedRef.current = true;
        showToast('System tray is unavailable on this desktop. Minimized to taskbar instead.', 'info');
      }
    } catch {
      await getCurrentWindow().minimize();
    }
  };

  const normalizeHotkey = (keys: Set<string>): string => {
    const modifiers: string[] = [];
    let primaryKey = '';

    keys.forEach(key => {
      const lower = key.toLowerCase();
      if (lower === 'control' || lower === 'controlleft' || lower === 'controlright') modifiers.push('Ctrl');
      else if (lower === 'shift' || lower === 'shiftleft' || lower === 'shiftright') modifiers.push('Shift');
      else if (lower === 'alt' || lower === 'altleft' || lower === 'altright') modifiers.push('Alt');
      else if (lower === 'meta' || lower === 'metaleft' || lower === 'metaright' || lower === 'osleft' || lower === 'osright') modifiers.push('Super');
      else if (key.startsWith('Key')) {
        // Handle KeyA, KeyB, etc.
        primaryKey = key.slice(3); // "KeyA" -> "A"
      } else if (key === 'Space') {
        primaryKey = 'Space';
      } else {
        // Other keys like F1, Escape, etc.
        primaryKey = key.charAt(0).toUpperCase() + key.slice(1).toLowerCase();
      }
    });

    return [...modifiers.sort(), primaryKey].filter(Boolean).join('+');
  };

  const setRecordingState = async (isRecording: boolean) => {
    setIsRecordingHotkey(isRecording);
    try {
      await invoke('set_configuring_hotkey', { isConfiguring: isRecording });
    } catch (e) {
      console.error('Failed to sync configuring hotkey state', e);
    }
  };

  const cancelHotkeyCapture = async () => {
    await setRecordingState(false);
    setRecordedKeys(new Set());
    setShowHotkeyCaptureModal(false);
    showToast('Hotkey configuration cancelled.', 'info');
  };

  const handleHotkeyKeyDown = (e: KeyboardEvent) => {
    if (!isRecordingHotkey) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.repeat) return;

    if (e.key === 'Escape') {
      void cancelHotkeyCapture();
      return;
    }

    const newKeys = new Set(recordedKeys);
    if (e.ctrlKey) newKeys.add('Control');
    if (e.shiftKey) newKeys.add('Shift');
    if (e.altKey) newKeys.add('Alt');
    if (e.metaKey) newKeys.add('Meta');
    
    const code = e.code;
    const modifierCodes = [
      'ControlLeft',
      'ControlRight',
      'ShiftLeft',
      'ShiftRight',
      'AltLeft',
      'AltRight',
      'MetaLeft',
      'MetaRight',
      'OSLeft',
      'OSRight',
    ];

    if (!modifierCodes.includes(code)) {
      newKeys.add(code);
      const normalized = normalizeHotkey(newKeys).toLowerCase();
      if (!normalized || ['ctrl', 'shift', 'alt', 'super'].includes(normalized)) {
        showToast('Please include a non-modifier key in the shortcut.', 'error');
        setRecordedKeys(newKeys);
        return;
      }
      void applyCapturedHotkey(normalized);
    } else {
      setRecordedKeys(newKeys);
    }
  };

  const handleHotkeyKeyUp = (e: KeyboardEvent) => {
    if (!isRecordingHotkey) return;
    e.preventDefault();
    e.stopPropagation();
  };

  const isAllReady = isPortalSetupReady && isAudioDeviceReady && isLocalModelReady;
  const startupChecksLoaded = hasLoadedConfig && hasLoadedSetupStatus && hasLoadedMics && hasLoadedModels;

  useEffect(() => {
    if (initialRouteChecked || !startupChecksLoaded) {
      return;
    }

    const hasExplicitRoute = hashHasExplicitRoute(window.location.hash);
    const currentHashRoute = routeFromHash(window.location.hash);

    if (currentHashRoute === 'ui-lab') {
      setInitialRouteChecked(true);
      return;
    }

    if (isAllReady) {
      if (!hasExplicitRoute || currentHashRoute === 'setup') {
        navigate('status', true);
      }
    } else if (!hasExplicitRoute || currentHashRoute !== 'setup') {
      navigate('setup', true);
    }

    setInitialRouteChecked(true);
  }, [initialRouteChecked, startupChecksLoaded, isAllReady]);

  const handleTitleBarMouseDown = async (event: MouseEvent) => {
    const target = event.target as HTMLElement | null;
    if (event.detail > 1) {
      event.preventDefault();
      return;
    }

    if (event.buttons === 1 && !target?.closest('button')) {
      event.preventDefault();
      await getCurrentWindow().startDragging();
    }
  };

  const handleTitleBarDoubleClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement | null;
    if (target?.closest('button')) {
      return;
    }

    event.preventDefault();

    await toggleWindowMaximize();
  };

  const toggleWindowMaximize = async () => {
    try {
      const win = getCurrentWindow();
      if (await win.isMaximized()) {
        await win.unmaximize();
      } else {
        await win.maximize();
      }
    } catch {
      // no-op if maximize is unavailable
    }
  };

  const handleSetActiveConfigSection = (value: string | null) => {
    setActiveConfigSection(value);
    if (tabContentRef.current) {
      tabContentRef.current.scrollTop = 0;
    }
  };

  useTauriEvents({
    onSetupStatus: (payload) => {
      if (payload === 'configuring-system') {
        showToast('Configuring system permissions...', 'info');
      } else if (payload === 'restart-required') {
        showToast('Permissions updated! Please restart your session.', 'success');
      } else if (payload === 'setup-failed') {
        showToast('System configuration failed.', 'error');
      }
    },
    onStatusUpdate: (payload) => {
      const nextStatus = typeof payload === 'string' ? payload : payload.status;
      setCurrentStatus(nextStatus);
      if (nextStatus === 'Error') {
        showToast('Mic not found — check your audio device settings.', 'error');
      }
    },
    onHistoryUpdated: () => { loadHistory(); },
    onConfigUpdated: () => { loadConfig(); },
    onHotkeyBindingState: setHotkeyBindingState,
    onMicTestStarted: () => { setMicTestStatus('playing'); },
    onMicTestFinished: () => {
      setMicTestStatus('idle');
      setMicVolume(0);
      setMicTestPassed(true);
    },
    onMicVolume: setMicVolume,
    onDownloadProgress: setDownloadProgress,
    onFocus: checkSetupStatus,
    onHashChange: () => {
      setActiveRoute(routeFromHash(window.location.hash));
    },
  });

  return (
    <div style={appShellStyle}>
      <TitleBar
        onMinimize={handleMinimize}
        onMaximize={() => void toggleWindowMaximize()}
        onClose={handleClose}
        onMouseDown={handleTitleBarMouseDown}
        onDoubleClick={handleTitleBarDoubleClick}
      />

      {activeRoute === 'setup' ? (
        <div style={appContentStyle}>
              <InitialSetupPage
                permissions={permissions}
                config={config}
                availableModels={availableModels}
                modelStatus={modelStatus}
                downloadProgress={downloadProgress}
                isDownloading={isDownloading}
                portalVersion={portalVersion}
                isSystemManagedShortcut={isSystemManagedShortcut}
                systemShortcutContext={systemShortcutContext}
                isApplyingHotkey={isApplyingHotkey}
                availableMics={availableMics}
                micTestStatus={micTestStatus}
                micVolume={micVolume}
                micTestPassed={micTestPassed}
                isLocalModelReady={isLocalModelReady}
                isAudioDeviceReady={isAudioDeviceReady}
                isAllReady={isAllReady}
                isRecordingHotkey={isRecordingHotkey}
                setupTouched={setupTouched}
                onTouchSetup={() => setSetupTouched(true)}
                onAudioSetup={() => void handleAudioSetup()}
                onInputSetup={() => void handleInputSetup()}
                onConfigureHotkey={() => void handleConfigureHotkey()}
                onHotkeyKeyDown={handleHotkeyKeyDown}
                onHotkeyKeyUp={handleHotkeyKeyUp}
                onHotkeyBlur={() => void setRecordingState(false)}
                onChangeConfig={updateConfig}
                onShowModelGuide={() => setShowModelGuide(true)}
                onDownloadModel={(size) => void downloadModel(size)}
                onRetryModels={() => void loadModels()}
                onLoadMics={() => void loadMics()}
                onStartMicTest={() => void startMicTest()}
                onStopMicTest={() => void stopMicTest()}
                onStopMicPlayback={() => void stopMicPlayback()}
                onRefreshStatus={() => void checkSetupStatus()}
                onFinishSetup={() => navigate('status')}
              />
            </div>
      ) : (
        <MainLayout
          activeRoute={activeRoute}
          config={config}
          currentStatus={currentStatus}
          appVersion={appVersion}
          availableEngines={availableEngines}
          availableModels={availableModels}
          modelStatus={modelStatus}
          downloadProgress={downloadProgress}
          isDownloading={isDownloading}
          isTestingApi={isTestingApi}
          activeConfigSection={activeConfigSection}
          portalVersion={portalVersion}
          isSystemManagedShortcut={isSystemManagedShortcut}
          hotkeyBindingState={hotkeyBindingState}
          isApplyingHotkey={isApplyingHotkey}
          availableMics={availableMics}
          micTestStatus={micTestStatus}
          micVolume={micVolume}
          overlayPositioningCapabilities={overlayPositioningCapabilities}
          checkingUpdates={checkingUpdates}
          autostartEnabled={autostartEnabled}
          hoveredTopTab={hoveredTopTab}
          history={history}
          updateResult={updateResult}
          tabContentRef={tabContentRef}
          onNavigate={navigate}
          onLogUI={logUI}
          onSetHoveredTab={setHoveredTopTab}
          onSetActiveConfigSection={handleSetActiveConfigSection}
          onUpdateConfig={updateConfig}
          onTestApiKey={testApiKey}
          onDownloadModel={downloadModel}
          onLoadModels={loadModels}
          onLoadMics={loadMics}
          onHandleConfigureHotkey={handleConfigureHotkey}
          onSetShowModelGuide={setShowModelGuide}
          onStartMicTest={startMicTest}
          onStopMicTest={stopMicTest}
          onStopMicPlayback={stopMicPlayback}
          onOpenDebugFolder={openDebugFolder}
          onReopenInitialSetup={() => { setSetupTouched(true); navigate('setup'); }}
          onFactoryReset={() => setShowFactoryResetModal(true)}
          onCheckForUpdates={() => void checkForUpdates(true)}
          onOpenUiLab={() => navigate('ui-lab')}
          onToggleAutostart={(enabled) => void toggleAutostart(enabled)}
          onCopyToClipboard={copyToClipboard}
          onClearHistory={clearHistory}
          onToggleOutputMethod={toggleOutputMethod}
          onOpenUpdateModal={() => setShowUpdateModal(true)}
        />
      )}

      <ToastContainer />

      <Modals
        showHotkeyCaptureModal={showHotkeyCaptureModal}
        showSystemShortcutModal={showSystemShortcutModal}
        showFactoryResetModal={showFactoryResetModal}
        showUpdateModal={showUpdateModal}
        showModelGuide={showModelGuide}
        isRecordingHotkey={isRecordingHotkey}
        isApplyingHotkey={isApplyingHotkey}
        configHotkey={config.hotkey}
        systemShortcutContext={systemShortcutContext}
        hotkeyBindingState={hotkeyBindingState}
        updateResult={updateResult}
        appVersion={appVersion}
        getLastCheckedLabel={getLastCheckedLabel}
        onCancelHotkeyCapture={() => void cancelHotkeyCapture()}
        onCloseSystemShortcut={() => setShowSystemShortcutModal(false)}
        onChangedSystemShortcut={() => {
          setShowSystemShortcutModal(false);
          void checkSetupStatus();
          void loadConfig();
        }}
        onCloseFactoryReset={() => setShowFactoryResetModal(false)}
        onFactoryReset={() => void handleFactoryReset()}
        onCloseUpdate={() => setShowUpdateModal(false)}
        onOpenLatestRelease={() => void openLatestReleasePage()}
        onCloseModelGuide={() => setShowModelGuide(false)}
      />
    </div>
  );
}

export default App;
