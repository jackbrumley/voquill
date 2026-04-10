
import { useState, useEffect, useRef } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { tokens } from './design-tokens.ts';
import { Card } from './components/Card.tsx';
import { Button } from './components/Button.tsx';
import { ActionFooter } from './components/ActionFooter.tsx';
import { ModelInfoModal } from './components/ModelInfoModal.tsx';
import { StatusPage } from './pages/StatusPage.tsx';
import { ConfigPage } from './pages/ConfigPage.tsx';
import { HistoryPage } from './pages/HistoryPage.tsx';
import { InitialSetupPage } from './pages/InitialSetupPage.tsx';
import './App.css';

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
  type: 'success' | 'error' | 'info';
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
  
  const [activeTab, setActiveTab] = useState<'status' | 'history' | 'config'>('status');
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
  const [linuxSetupStatus, setLinuxSetupStatus] = useState<'idle' | 'configuring' | 'restart-required' | 'failed'>('idle');
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [recordedKeys, setRecordedKeys] = useState<Set<string>>(new Set());
  const [showModelGuide, setShowModelGuide] = useState(false);
  const [portalVersion, setPortalVersion] = useState<number>(0);
  const [portalDiagnostics, setPortalDiagnostics] = useState<PortalDiagnostics | null>(null);
  const [hotkeyBindingState, setHotkeyBindingState] = useState<HotkeyBindingState | null>(null);
  const [showHotkeyCaptureModal, setShowHotkeyCaptureModal] = useState(false);
  const [isApplyingHotkey, setIsApplyingHotkey] = useState(false);
  const [showInitialSetup, setShowInitialSetup] = useState(true);
  const [setupTouched, setSetupTouched] = useState(false);
  const tabContentRef = useRef<HTMLDivElement | null>(null);
  const trayFallbackNotifiedRef = useRef(false);

  useEffect(() => {
    invoke<number>('get_wayland_portal_version')
      .then(setPortalVersion)
      .catch(e => console.log("Not running Wayland portal version check:", e));

    invoke<PortalDiagnostics>('get_portal_diagnostics')
      .then(setPortalDiagnostics)
      .catch(e => console.log('Portal diagnostics unavailable:', e));

    invoke<HotkeyBindingState>('get_hotkey_binding_state')
      .then(setHotkeyBindingState)
      .catch(e => console.log('Hotkey binding state unavailable:', e));
  }, []);

  const logUI = (msg: string) => {
    // Log Toasts and Clicks always, drop other spam unless debug mode
    if (!config.debug_mode && !msg.includes('Button clicked') && !msg.includes('Toast')) return;
    const timestamp = new Date().toLocaleTimeString();
    console.log(`[${timestamp}] ${msg}`);
    invoke('log_ui_event', { message: msg }).catch((err) => {
      console.error(`Failed to send log to backend: ${err}`);
    });
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

    const unlistenStatus = listen<string>('status-update', (event) => {
      setCurrentStatus(event.payload);
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
  }, [activeTab]);

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
    }
  };

  const loadMics = async () => {
    try {
      const devices = await invoke<AudioDevice[]>('get_audio_devices');
      setAvailableMics(devices);
    } catch (error) {
      showToast(`Failed to load microphones: ${error}`, 'error');
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
    }
  };

  const downloadModel = async (size: string) => {
    logUI(`🖱️ Button clicked: Download Model (${size})`);
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
    logUI('🖱️ Button clicked: Clear History');
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

  const persistConfig = async (configToPersist: Config) => {
    try {
      const configToSave = {
        ...configToPersist,
        typing_speed_interval: configToPersist.typing_speed_interval / 1000,
        openai_api_key: configToPersist.openai_api_key || 'your_api_key_here',
      };
      await invoke('save_config', { newConfig: configToSave });
    } catch (error) {
      console.error('Failed to auto-save configuration:', error);
      showToast(`Failed to save: ${error}`, 'error');
    }
  };

  useEffect(() => {
    const timer = setTimeout(() => {
      persistConfig(config);
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
    logUI('🖱️ Button clicked: Test Microphone (Start)');
    try {
      setMicTestStatus('recording');
      await invoke('start_mic_test');
    } catch (error) {
      setMicTestStatus('idle');
      showToast(`Failed to start mic test: ${error}`, 'error');
    }
  };

  const stopMicTest = async () => {
    logUI('🖱️ Button clicked: Stop & Play Back');
    setMicTestStatus('processing');
    try {
      await invoke('stop_mic_test');
    } catch (error) {
      setMicTestStatus('idle');
      showToast(`Failed to stop mic test: ${error}`, 'error');
    }
  };

  const stopMicPlayback = async () => {
    logUI('🖱️ Button clicked: Stop Playback');
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

  const openDebugFolder = async () => {
    logUI('🖱️ Button clicked: Open Debug Folder');
    try {
      await invoke('open_debug_folder');
    } catch (error) {
      showToast('Failed to open debug folder', 'error');
    }
  };

  const testApiKey = async () => {
    logUI('🖱️ Button clicked: Test Key');
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

  const showToast = (message: string, type: 'success' | 'error' | 'info' = 'info') => {
    // Log to console/backend
    const emoji = type === 'success' ? '✅' : type === 'error' ? '❌' : 'ℹ️';
    logUI(`${emoji} Toast: ${message}`);

    const id = Date.now();
    setToasts(prev => [...prev, { id, message, type }]);
    
    // Errors stay longer (10s), others 3s
    const duration = type === 'error' ? 10000 : 3000;
    
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, duration);
  };

  const handleToastClick = async (toast: Toast) => {
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
      const logs = await invoke<string>('get_session_log_text');
      await invoke('plugin:clipboard-manager|write_text', { text: logs });
      showToast('Session logs copied to clipboard.', 'success');
    } catch (error) {
      showToast(`Failed to copy session logs: ${error}`, 'error');
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
      else if (lower === 'meta' || lower === 'metaleft' || lower === 'metaright') modifiers.push('Super');
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
    
    if (!['ControlLeft', 'ControlRight', 'ShiftLeft', 'ShiftRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight'].includes(code)) {
      newKeys.add(code);
      const normalized = normalizeHotkey(newKeys).toLowerCase();
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

  useEffect(() => {
    if (permissions && isAllReady && !setupTouched) {
      setShowInitialSetup(false);
    }
  }, [permissions, isAllReady, setupTouched]);

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

  return (
    <div className="app">
      <div className="title-bar" onMouseDown={handleTitleBarMouseDown}>
        <div className="title-bar-title">Voquill</div>
        <div className="title-bar-controls">
          <Button variant="icon" className="title-bar-button" onClick={handleMinimize}>─</Button>
          <Button variant="icon" className="title-bar-button close" onClick={handleClose}>✕</Button>
        </div>
      </div>

      {showInitialSetup ? (
        <InitialSetupPage
          permissions={permissions}
          config={config}
          availableModels={availableModels}
          modelStatus={modelStatus}
          downloadProgress={downloadProgress}
          isDownloading={isDownloading}
          portalVersion={portalVersion}
          portalDiagnostics={portalDiagnostics}
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
          onFinishSetup={() => setShowInitialSetup(false)}
        />
      ) : (
        <>
          <div className="tab-nav">
            <button className={`tab ${activeTab === 'status' ? 'active' : ''}`} onClick={() => setActiveTab('status')}>Status</button>
            <button className={`tab ${activeTab === 'history' ? 'active' : ''}`} onClick={() => setActiveTab('history')}>History</button>
            <button className={`tab ${activeTab === 'config' ? 'active' : ''}`} onClick={() => setActiveTab('config')}>Config</button>
          </div>

          <div className="tab-content" ref={tabContentRef}>
            {activeTab === 'status' && (
              <StatusPage
                currentStatus={currentStatus}
                appVersion={appVersion}
                modelStatus={modelStatus}
                config={config}
                onToggleOutputMethod={toggleOutputMethod}
              />
            )}

            {activeTab === 'config' && (
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
                onReopenInitialSetup={() => {
                  setSetupTouched(true);
                  setShowInitialSetup(true);
                }}
                onCopySessionLogs={() => void copySessionLogs()}
              />
            )}

            {activeTab === 'history' && (
              <HistoryPage history={history} onCopyToClipboard={copyToClipboard} />
            )}
          </div>

          {activeTab === 'history' && (
            <ActionFooter>
              <Button variant="danger" className="sticky-footer-button" onClick={clearHistory}>Clear History</Button>
            </ActionFooter>
          )}
        </>
      )}

      <div className="toast-container">
        {toasts.map(toast => (
          <div
            key={toast.id}
            className={`toast ${toast.type}`}
            title="Click to copy"
            onClick={() => void handleToastClick(toast)}
          >
            <span className="toast-dot"></span>
            <span className="toast-message">{toast.message}</span>
          </div>
        ))}
      </div>

      {showHotkeyCaptureModal && (
        <div className="hotkey-capture-overlay">
          <Card className="hotkey-capture-card">
            <div className="hotkey-capture-content">
              <h3 className="hotkey-capture-title">Configure Hotkey</h3>
              <p className="hotkey-capture-subtitle">
                Press your desired key combination, or press Escape to cancel.
              </p>
              <div className="hotkey-capture-display">
                {isRecordingHotkey ? 'Listening for keys...' : config.hotkey}
              </div>
              <div className="hotkey-capture-actions">
                <Button
                  variant="ghost"
                  onClick={() => void cancelHotkeyCapture()}
                  disabled={isApplyingHotkey}
                >
                  Cancel
                </Button>
              </div>
            </div>
          </Card>
        </div>
      )}

      {showModelGuide && <ModelInfoModal onClose={() => setShowModelGuide(false)} />}
    </div>
  );
}

export default App;
