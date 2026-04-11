
import { useState, useEffect, useRef } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { Button } from './components/Button.tsx';
import { ActionFooter } from './components/ActionFooter.tsx';
import { ModelInfoModal } from './components/ModelInfoModal.tsx';
import { Modal } from './components/Modal.tsx';
import { StatusPage } from './pages/StatusPage.tsx';
import { ConfigPage } from './pages/ConfigPage.tsx';
import { HistoryPage } from './pages/HistoryPage.tsx';
import { InitialSetupPage } from './pages/InitialSetupPage.tsx';
import {
  appShellStyle,
  helperTextStyle,
  modalShortcutNoteStyle,
  modalShortcutPathStyle,
  modalTextIntroStyle,
  tabContentStyle,
  tabNavStyle,
  titleBarControlsStyle,
  titleBarStyle,
  titleBarTitleStyle,
  toastContainerStyle,
  getToastMessageStyle,
  getToastStyle,
} from './theme/ui-primitives.ts';
import { tokens } from './design-tokens.ts';

interface Config {
  openai_api_key: string;
  api_url: string;
  api_model: string;
  transcription_mode: 'API' | 'Local';
  local_model_size: string;
  local_engine: string;
  hotkey: string;
  typing_speed_interval: number;
  key_press_duration_ms: number;
  pixels_from_bottom: number;
  audio_device: string | null;
  debug_mode: boolean;
  enable_recording_logs: boolean;
  input_sensitivity: number;
  output_method: 'Typewriter' | 'Clipboard';
  copy_on_typewriter: boolean;
  language: string;
  enable_gpu: boolean;
  shortcuts_token?: string;
  input_token?: string;
}

interface Toast {
  id: number;
  message: string;
  type: 'success' | 'error' | 'info' | 'saved';
}

interface HistoryItem {
  id: number;
  text: string;
  timestamp: string;
}

interface AudioDevice {
  id: string;
  label: string;
}

interface LinuxPermissions {
  audio: boolean;
  shortcuts: boolean;
  input_emulation: boolean;
  shortcuts_status: string;
  shortcuts_detail?: string;
}

interface PortalDiagnostics {
  available: boolean;
  version: number;
  supports_configure_shortcuts: boolean;
  has_record_shortcut: boolean;
  active_trigger?: string;
  status: string;
  detail?: string;
}

interface ConfigureHotkeyResult {
  outcome: 'configured' | 'requires_in_app_capture';
  detail?: string;
}

interface HotkeyBindingState {
  bound: boolean;
  listening: boolean;
  detail?: string;
  active_trigger?: string;
}

interface SystemShortcutContext {
  distro?: string;
  desktop?: string;
  settings_path: string;
}

interface StatusUpdatePayload {
  seq: number;
  status: string;
}

type AppRoute = 'setup' | 'status' | 'history' | 'config';

const DEFAULT_ROUTE: AppRoute = 'status';

const routeFromHash = (hash: string): AppRoute => {
  const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
  if (normalized === 'setup' || normalized === 'status' || normalized === 'history' || normalized === 'config') {
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
  });
  
  const [activeRoute, setActiveRoute] = useState<AppRoute>(routeFromHash(window.location.hash));
  const [isTestingApi, setIsTestingApi] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [availableMics, setAvailableMics] = useState<AudioDevice[]>([]);
  const [micTestStatus, setMicTestStatus] = useState<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const [micVolume, setMicVolume] = useState<number>(0);
  const [micTestPassed, setMicTestPassed] = useState(false);
  const [activeConfigSection, setActiveConfigSection] = useState<string | null>(null);
  const [appVersion, setAppVersion] = useState<string>('');
  const [availableEngines, setAvailableEngines] = useState<string[]>([]);
  const [availableModels, setAvailableModels] = useState<any[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<number>(0);
  const [isDownloading, setIsDownloading] = useState(false);
  const [modelStatus, setModelStatus] = useState<Record<string, boolean>>({});
  const [permissions, setPermissions] = useState<LinuxPermissions | null>(null);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [recordedKeys, setRecordedKeys] = useState<Set<string>>(new Set());
  const [showModelGuide, setShowModelGuide] = useState(false);
  const [portalVersion, setPortalVersion] = useState<number>(0);
  const [portalDiagnostics, setPortalDiagnostics] = useState<PortalDiagnostics | null>(null);
  const [hotkeyBindingState, setHotkeyBindingState] = useState<HotkeyBindingState | null>(null);
  const [systemShortcutContext, setSystemShortcutContext] = useState<SystemShortcutContext | null>(null);
  const [showHotkeyCaptureModal, setShowHotkeyCaptureModal] = useState(false);
  const [showSystemShortcutModal, setShowSystemShortcutModal] = useState(false);
  const [showFactoryResetModal, setShowFactoryResetModal] = useState(false);
  const [isApplyingHotkey, setIsApplyingHotkey] = useState(false);
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
    const syncRouteFromHash = () => {
      setActiveRoute(routeFromHash(window.location.hash));
    };

    window.addEventListener('hashchange', syncRouteFromHash);

    invoke<number>('get_wayland_portal_version')
      .then(setPortalVersion)
      .catch(e => console.log("Not running Wayland portal version check:", e));

    invoke<PortalDiagnostics>('get_portal_diagnostics')
      .then(setPortalDiagnostics)
      .catch(e => console.log('Portal diagnostics unavailable:', e));

    invoke<HotkeyBindingState>('get_hotkey_binding_state')
      .then(setHotkeyBindingState)
      .catch(e => console.log('Hotkey binding state unavailable:', e));

    invoke<SystemShortcutContext>('get_system_shortcut_context')
      .then(setSystemShortcutContext)
      .catch(e => console.log('System shortcut context unavailable:', e));

    syncRouteFromHash();

    return () => {
      window.removeEventListener('hashchange', syncRouteFromHash);
    };
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

    const unlistenPressed = listen('hotkey-pressed', () => {
      setCurrentStatus('Recording');
    });

    const unlistenReleased = listen('hotkey-released', () => {
      setCurrentStatus('Transcribing');
    });

    const unlistenSetup = listen<string>('setup-status', (event) => {
      if (event.payload === 'configuring-system') {
        showToast('Configuring system permissions...', 'info');
      } else if (event.payload === 'restart-required') {
        showToast('Permissions updated! Please restart your session.', 'success');
      } else if (event.payload === 'setup-failed') {
        showToast('System configuration failed.', 'error');
      }
    });

    const unlistenStatus = listen<string | StatusUpdatePayload>('status-update', (event) => {
      const payload = event.payload;
      const nextStatus = typeof payload === 'string' ? payload : payload.status;
      setCurrentStatus(nextStatus);
    });

    const unlistenHistory = listen('history-updated', () => {
      loadHistory();
    });

    const unlistenConfigUpdated = listen('config-updated', () => {
      loadConfig();
    });

    const unlistenHotkeyBindingState = listen<HotkeyBindingState>('hotkey-binding-state', (event) => {
      setHotkeyBindingState(event.payload);
    });
    
    const unlistenMicTestStarted = listen('mic-test-playback-started', () => {
      setMicTestStatus('playing');
    });

    const unlistenMicTestFinished = listen('mic-test-playback-finished', () => {
      setMicTestStatus('idle');
      setMicVolume(0);
      setMicTestPassed(true);
    });

    const unlistenMicVolume = listen<number>('mic-test-volume', (event: any) => {
      setMicVolume(event.payload as number);
    });

    const unlistenDownloadProgress = listen<number>('model-download-progress', (event: any) => {
      setDownloadProgress(event.payload as number);
    });

    const onFocus = () => {
      checkSetupStatus();
    };
    window.addEventListener('focus', onFocus);

    return () => {
      window.removeEventListener('focus', onFocus);
      unlistenPressed.then((fn: any) => fn());
      unlistenReleased.then((fn: any) => fn());
      unlistenSetup.then((fn: any) => fn());
      unlistenStatus.then((fn: any) => fn());
      unlistenHistory.then((fn: any) => fn());
      unlistenConfigUpdated.then((fn: any) => fn());
      unlistenHotkeyBindingState.then((fn: any) => fn());
      unlistenMicTestStarted.then((fn: any) => fn());
      unlistenMicTestFinished.then((fn: any) => fn());
      unlistenMicVolume.then((fn: any) => fn());
      unlistenDownloadProgress.then((fn: any) => fn());
    };
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
      const diagnostics = await invoke<PortalDiagnostics>('get_portal_diagnostics');
      setPortalDiagnostics(diagnostics);
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

    if (isSystemManagedShortcut) {
      setShowSystemShortcutModal(true);
      return;
    }

    try {
      setIsApplyingHotkey(true);
      const result = await invoke<ConfigureHotkeyResult>('configure_hotkey');

      if (result.outcome === 'requires_in_app_capture') {
        setShowHotkeyCaptureModal(true);
        await setRecordingState(true);
        setRecordedKeys(new Set());
        showToast('Press your desired key combination in the modal.', 'info');
      } else {
        showToast('Shortcut configured successfully!', 'success');
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
      const savedHistory = await invoke<any>('get_history');
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

      const models = await invoke<any[]>('get_available_models');
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
    } catch (error) {
      showToast('Failed to clear history', 'error');
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await invoke('plugin:clipboard-manager|write_text', { text });
      showToast('Copied to clipboard', 'success');
    } catch (error) {
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
      persistConfig(config, hasChanges && previousConfig !== null);
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

  const updateConfig = (key: string, value: any) => {
    setConfig(prev => ({ ...prev, [key]: value } as Config));
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
  const isSystemManagedShortcut =
    !!portalDiagnostics?.available &&
    portalDiagnostics.version >= 1 &&
    !portalDiagnostics.supports_configure_shortcuts;

  const openDebugFolder = async () => {
    try {
      await invoke('open_debug_folder');
    } catch (error) {
      showToast('Failed to open debug folder', 'error');
    }
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

  const showToast = (message: string, type: 'success' | 'error' | 'info' | 'saved' = 'info') => {
    // Log to console/backend
    const emoji = type === 'success' ? '✅' : type === 'error' ? '❌' : type === 'saved' ? '💾' : 'ℹ️';
    logUI(`${emoji} Toast: ${message}`);

    const id = Date.now();
    setToasts(prev => {
      if (type === 'saved') {
        return [...prev.filter(toast => toast.type !== 'saved'), { id, message, type }];
      }
      return [...prev, { id, message, type }];
    });
    
    // Errors stay longer (10s), saved confirmations are brief, others 3s
    const duration = type === 'error' ? 10000 : type === 'saved' ? 900 : 3000;
    
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, duration);
  };

  const handleToastClick = async (toast: Toast) => {
    if (toast.type === 'saved') {
      setToasts(prev => prev.filter(t => t.id !== toast.id));
      return;
    }

    try {
      await invoke('plugin:clipboard-manager|write_text', { text: toast.message });
    } catch (error) {
      console.error('Failed to copy toast message:', error);
    } finally {
      setToasts(prev => prev.filter(t => t.id !== toast.id));
    }
  };

  const copySessionLogs = async () => {
    try {
      await invoke('copy_session_log_to_clipboard');
      showToast('Log copied to clipboard.', 'success');
    } catch (error) {
      showToast(`Failed to copy log: ${error}`, 'error');
    }
  };

  const openSessionLog = async () => {
    try {
      await invoke('open_session_log');
    } catch (error) {
      showToast(`Failed to open log file: ${error}`, 'error');
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

    if (isAllReady) {
      if (!hasExplicitRoute || currentHashRoute === 'setup') {
        navigate('status', true);
      }
    } else if (!hasExplicitRoute || currentHashRoute !== 'setup') {
      navigate('setup', true);
    }

    setInitialRouteChecked(true);
  }, [initialRouteChecked, startupChecksLoaded, isAllReady]);

  const handleTitleBarMouseDown = async (e: any) => {
    if (e.buttons === 1 && !e.target.closest('button')) {
      await getCurrentWindow().startDragging();
    }
  };

  const handleSetActiveConfigSection = (value: string | null) => {
    setActiveConfigSection(value);
    if (tabContentRef.current) {
      tabContentRef.current.scrollTop = 0;
    }
  };

  const topTabBaseStyle = {
    border: 'none',
    borderRadius: `${tokens.radii.input} ${tokens.radii.input} 0 0`,
    background: 'transparent',
    color: tokens.colors.textSecondary,
    fontSize: '12px',
    fontWeight: 600,
    letterSpacing: '0.005em',
    padding: `12px ${tokens.spacing.sm}`,
    cursor: 'pointer',
    transition: tokens.transitions.normal,
    flex: 1,
    textAlign: 'center',
    position: 'relative',
    zIndex: 1,
    marginBottom: 0,
  } as const;

  const getTopTabStyle = (route: AppRoute) => {
    const isActive = activeRoute === route;
    const isHovered = hoveredTopTab === route;
    return {
      ...topTabBaseStyle,
      background: isActive
        ? 'rgba(54, 57, 63, 0.5)'
        : isHovered
          ? 'rgba(255, 255, 255, 0.05)'
          : 'transparent',
      color: isActive ? tokens.colors.textPrimary : tokens.colors.textSecondary,
      backdropFilter: isActive ? 'blur(5px)' : undefined,
      WebkitBackdropFilter: isActive ? 'blur(5px)' : undefined,
      boxShadow: isActive ? `inset 0 -1px 0 ${tokens.colors.bgPrimary}` : 'none',
    } as const;
  };

  return (
    <div style={appShellStyle}>
      <div style={titleBarStyle} onMouseDown={handleTitleBarMouseDown}>
        <div style={titleBarTitleStyle}>Voquill</div>
        <div style={titleBarControlsStyle}>
          <Button variant="titlebarIcon" onClick={handleMinimize}>─</Button>
          <Button variant="titlebarClose" onClick={handleClose}>✕</Button>
        </div>
      </div>

      {activeRoute === 'setup' ? (
        <InitialSetupPage
          permissions={permissions}
          config={config}
          availableModels={availableModels}
          modelStatus={modelStatus}
          downloadProgress={downloadProgress}
          isDownloading={isDownloading}
          portalVersion={portalVersion}
          portalDiagnostics={portalDiagnostics}
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
      ) : (
        <>
          <div style={tabNavStyle}>
            <button
              type="button"
              style={getTopTabStyle('status')}
              onClick={() => { logUI('🖱️ Button clicked: Status Tab'); navigate('status'); }}
              onMouseEnter={() => setHoveredTopTab('status')}
              onMouseLeave={() => setHoveredTopTab(null)}
              aria-current={activeRoute === 'status' ? 'page' : undefined}
            >
              Status
            </button>
            <button
              type="button"
              style={getTopTabStyle('history')}
              onClick={() => { logUI('🖱️ Button clicked: History Tab'); navigate('history'); }}
              onMouseEnter={() => setHoveredTopTab('history')}
              onMouseLeave={() => setHoveredTopTab(null)}
              aria-current={activeRoute === 'history' ? 'page' : undefined}
            >
              History
            </button>
            <button
              type="button"
              style={getTopTabStyle('config')}
              onClick={() => { logUI('🖱️ Button clicked: Config Tab'); navigate('config'); }}
              onMouseEnter={() => setHoveredTopTab('config')}
              onMouseLeave={() => setHoveredTopTab(null)}
              aria-current={activeRoute === 'config' ? 'page' : undefined}
            >
              Config
            </button>
          </div>

          <div style={tabContentStyle} ref={tabContentRef}>
            {activeRoute === 'status' && (
              <StatusPage
                currentStatus={currentStatus}
                appVersion={appVersion}
                modelStatus={modelStatus}
                config={config}
                isSystemManagedShortcut={isSystemManagedShortcut}
                onToggleOutputMethod={toggleOutputMethod}
              />
            )}

            {activeRoute === 'config' && (
              <ConfigPage
                config={config}
                activeConfigSection={activeConfigSection}
                setActiveConfigSection={handleSetActiveConfigSection}
                availableEngines={availableEngines}
                availableModels={availableModels}
                modelStatus={modelStatus}
                downloadProgress={downloadProgress}
                isDownloading={isDownloading}
                isTestingApi={isTestingApi}
                portalVersion={portalVersion}
                portalDiagnostics={portalDiagnostics}
                isSystemManagedShortcut={isSystemManagedShortcut}
                systemShortcutContext={systemShortcutContext}
                hotkeyBindingState={hotkeyBindingState}
                isApplyingHotkey={isApplyingHotkey}
                availableMics={availableMics}
                micTestStatus={micTestStatus}
                micVolume={micVolume}
                updateConfig={updateConfig}
                testApiKey={testApiKey}
                downloadModel={downloadModel}
                loadModels={loadModels}
                loadMics={loadMics}
                handleConfigureHotkey={handleConfigureHotkey}
                setShowModelGuide={setShowModelGuide}
                startMicTest={() => void startMicTest()}
                stopMicTest={() => void stopMicTest()}
                stopMicPlayback={() => void stopMicPlayback()}
                openDebugFolder={openDebugFolder}
                openSessionLog={() => void openSessionLog()}
                onReopenInitialSetup={() => {
                  setSetupTouched(true);
                  navigate('setup');
                }}
                onCopySessionLogs={() => void copySessionLogs()}
                onFactoryReset={() => setShowFactoryResetModal(true)}
              />
            )}

            {activeRoute === 'history' && (
              <HistoryPage history={history} onCopyToClipboard={copyToClipboard} />
            )}
          </div>

          {activeRoute === 'history' && (
            <ActionFooter>
              <Button variant="danger" pill floating onClick={clearHistory}>Clear History</Button>
            </ActionFooter>
          )}
        </>
      )}

      <div style={toastContainerStyle}>
        {toasts.map(toast => (
          <div
            key={toast.id}
            style={getToastStyle(toast.type)}
            title={toast.type === 'saved' ? undefined : 'Click to copy'}
            onClick={() => void handleToastClick(toast)}
          >
            <span style={getToastMessageStyle(toast.type)}>{toast.message}</span>
          </div>
        ))}
      </div>

      {showHotkeyCaptureModal && (
        <Modal
          title="Configure Hotkey"
          onClose={() => void cancelHotkeyCapture()}
          maxWidth="440px"
          footer={
            <Button
              variant="ghost"
              onClick={() => void cancelHotkeyCapture()}
              disabled={isApplyingHotkey}
            >
              Cancel
            </Button>
          }
        >
          <p style={helperTextStyle}>
            Press your desired key combination, or press Escape to cancel.
          </p>
          <div style={{ border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', padding: '10px 12px', textAlign: 'center', fontWeight: 700 }}>
            {isRecordingHotkey ? 'Listening for keys...' : config.hotkey}
          </div>
        </Modal>
      )}

      {showSystemShortcutModal && (
        <Modal
          title="Change Shortcut"
          onClose={() => setShowSystemShortcutModal(false)}
          maxWidth="560px"
          footer={
            <>
              <Button variant="secondary" onClick={() => setShowSystemShortcutModal(false)}>
                Close
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  void (async () => {
                    setShowSystemShortcutModal(false);
                    await checkSetupStatus();
                    await loadConfig();
                  })();
                }}
              >
                I changed it
              </Button>
            </>
          }
        >
          <p style={modalTextIntroStyle}>
            Looks like your distro manages your shortcut. In order to change it, you will need to do so in your system settings.
          </p>
          <p style={modalShortcutPathStyle}>
            {systemShortcutContext?.settings_path || 'Settings -> Apps -> Voquill -> Global Shortcuts'}
          </p>
          <p style={modalShortcutNoteStyle}>
            If you can&apos;t find it, you may need to search through your system settings for &quot;Voquill&quot; or &quot;shortcuts&quot;.
          </p>
        </Modal>
      )}

      {showFactoryResetModal && (
        <Modal
          title="Factory Reset"
          onClose={() => setShowFactoryResetModal(false)}
          maxWidth="560px"
          footer={
            <>
              <Button variant="ghost" pill onClick={() => setShowFactoryResetModal(false)}>
                Cancel
              </Button>
              <Button variant="danger" pill onClick={() => void handleFactoryReset()}>
                Reset Everything
              </Button>
            </>
          }
        >
          <p style={modalTextIntroStyle}>
            This will reset Voquill to defaults and permanently clear downloaded models, logs, and history.
          </p>
          <p style={modalShortcutNoteStyle}>This action cannot be undone.</p>
        </Modal>
      )}

      {showModelGuide && <ModelInfoModal onClose={() => setShowModelGuide(false)} />}
    </div>
  );
}

export default App;
