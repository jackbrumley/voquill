
import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';
import { IconBrandGithub, IconHeart, IconMicrophone, IconKeyboard, IconTextRecognition, IconCheck, IconX, IconInfoCircle, IconRocket, IconRefresh, IconCopy, IconShieldLock } from '@tabler/icons-preact';
import StatusIcon from './StatusIcon.tsx';
import { tokens } from './design-tokens.ts';
import { Card } from './components/Card.tsx';
import { Button } from './components/Button.tsx';
import { ConfigField } from './components/ConfigField.tsx';
import { Switch } from './components/Switch.tsx';
import { CollapsibleSection } from './components/CollapsibleSection.tsx';
import { ModeSwitcher } from './components/ModeSwitcher.tsx';
import { ActionFooter } from './components/ActionFooter.tsx';
import { ModelInfoModal } from './components/ModelInfoModal.tsx';
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
  const [activeConfigSection, setActiveConfigSection] = useState<string | null>('basic');
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
    try {
      await invoke('request_audio_permission');
      showToast('Audio permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get audio permission: ${error}`, 'error');
    }
  };

  const handleInputSetup = async () => {
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
      await navigator.clipboard.writeText(text);
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

  const updateConfig = (key: keyof Config, value: any) => {
    setConfig(prev => ({ ...prev, [key]: value }));
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
      await navigator.clipboard.writeText(toast.message);
    } catch (error) {
      console.error('Failed to copy toast message:', error);
    } finally {
      setToasts(prev => prev.filter(t => t.id !== toast.id));
    }
  };

  const handleClose = async () => {
    await getCurrentWindow().hide();
  };

  const handleMinimize = async () => {
    await getCurrentWindow().minimize();
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

  const isAllReady = permissions && permissions.audio && permissions.shortcuts && permissions.input_emulation;

  const handleTitleBarMouseDown = async (e: any) => {
    if (e.buttons === 1 && !e.target.closest('button')) {
      await getCurrentWindow().startDragging();
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

      {!isAllReady ? (
        <div className="tab-panel-padded" style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Card className="setup-required-card">
            <div className="setup-header">
              <div className="setup-icon-container">
                <IconShieldLock size={32} className="setup-icon" />
              </div>
              <h2>System Access Required</h2>
            </div>
            
            <div className="setup-body">
              <p style={{ textAlign: 'center' }}>Voquill needs standard Wayland portal permissions to operate:</p>
              <div className="setup-list" style={{ width: '100%' }}>
                
                {/* Audio Permission */}
                <div className={`permission-item ${permissions?.audio ? 'ready' : ''}`}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px', textAlign: 'left', flex: 1 }}>
                    <div className="permission-icon">
                      <IconMicrophone size={20} />
                    </div>
                    <div className="permission-info">
                      <div className="permission-title">Audio Access</div>
                      <div className="permission-desc">Required for dictation</div>
                    </div>
                  </div>
                  <div className="permission-status" style={{ marginLeft: 'auto' }}>
                    {permissions?.audio ? (
                      <IconCheck color="var(--colors-success)" size={20} />
                    ) : (
                      <Button variant="ghost" size="sm" onClick={handleAudioSetup}>Request</Button>
                    )}
                  </div>
                </div>

                {/* Shortcuts Permission */}
                <div className={`permission-item ${permissions?.shortcuts ? 'ready' : ''}`}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px', textAlign: 'left', flex: 1 }}>
                    <div className="permission-icon">
                      <IconKeyboard size={20} />
                    </div>
                    <div className="permission-info" style={{ width: '100%', paddingRight: '10px' }}>
                      <div className="permission-title" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        Global Shortcuts
                        {!permissions?.shortcuts && (
                          <input 
                            type="text" 
                            className="hotkey-input setup-hotkey-input"
                            value={isRecordingHotkey ? 'Press keys...' : config.hotkey}
                            onKeyDown={handleHotkeyKeyDown}
                            onKeyUp={handleHotkeyKeyUp}
                            onFocus={() => null}
                            onBlur={() => setRecordingState(false)}
                            readOnly
                            placeholder={portalVersion >= 1 ? "Bind with button" : "Click to set"}
                            style={{ 
                              width: '140px', padding: '4px 8px', fontSize: '12px', 
                              backgroundColor: 'var(--colors-surface-active)', 
                              border: '1px solid var(--colors-border)', 
                              borderRadius: '4px', cursor: portalVersion >= 1 ? 'default' : 'pointer', textAlign: 'center',
                              color: isRecordingHotkey ? 'var(--colors-primary)' : 'var(--colors-text)',
                              opacity: portalVersion >= 1 ? 0.8 : 1
                            }}
                            title={portalVersion >= 1 ? 'Use Configure Hotkey to request a system shortcut.' : ''}
                          />
                        )}
                      </div>
                      <div className="permission-desc">Required for the hotkey</div>
                      {!permissions?.shortcuts && permissions?.shortcuts_detail && (
                        <div className="permission-desc" style={{ marginTop: '2px', color: 'var(--colors-warning)' }}>
                          {permissions.shortcuts_detail}
                        </div>
                      )}
                    </div>
                  </div>
                  <div className="permission-status">
                    {permissions?.shortcuts ? (
                      <IconCheck color="var(--colors-success)" size={20} />
                    ) : (
                      <Button variant="ghost" size="sm" onClick={handleConfigureHotkey} disabled={isApplyingHotkey}>Configure Hotkey</Button>
                    )}
                  </div>
                </div>

                {/* Input Simulation Permission */}
                <div className={`permission-item ${permissions?.input_emulation ? 'ready' : ''}`}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px', textAlign: 'left', flex: 1 }}>
                    <div className="permission-icon">
                      <IconTextRecognition size={20} />
                    </div>
                    <div className="permission-info">
                      <div className="permission-title">Input Simulation</div>
                      <div className="permission-desc">Required to type into other apps</div>
                    </div>
                  </div>
                  <div className="permission-status" style={{ marginLeft: 'auto' }}>
                    {permissions?.input_emulation ? (
                      <IconCheck color="var(--colors-success)" size={20} />
                    ) : (
                      <Button variant="ghost" size="sm" onClick={handleInputSetup}>Request</Button>
                    )}
                  </div>
                </div>

              </div>
              <p className="setup-note">
                Click "Request" or "Configure Hotkey" for each item to trigger the OS permission prompts.
                {portalDiagnostics && portalDiagnostics.available && (
                  <> Portal v{portalDiagnostics.version} ({portalDiagnostics.supports_configure_shortcuts ? 'configure supported' : 'bind/list only'}).</>
                )}
              </p>
            </div>

            <div className="setup-actions setup-button-container">
              <Button 
                variant="ghost" 
                onClick={checkSetupStatus} 
                size="sm"
                className="setup-button"
                style={{ width: '100%', marginTop: '10px' }}
              >
                Refresh Status
              </Button>
            </div>
          </Card>
        </div>
      ) : (
        <>
          <div className="tab-nav">
            <button className={`tab ${activeTab === 'status' ? 'active' : ''}`} onClick={() => setActiveTab('status')}>Status</button>
            <button className={`tab ${activeTab === 'history' ? 'active' : ''}`} onClick={() => setActiveTab('history')}>History</button>
            <button className={`tab ${activeTab === 'config' ? 'active' : ''}`} onClick={() => setActiveTab('config')}>Config</button>
          </div>

          <div className="tab-content">
            {activeTab === 'status' && (
              <div className="tab-panel" key="status">
                <div className="tab-panel-padded">
                  <div className="status-display">
                    <StatusIcon status={currentStatus} large />
                    <div className="status-text-app" key={`text-${currentStatus}`}>
                      {currentStatus === 'Transcribing' ? `Transcribing (${config.transcription_mode})` : currentStatus}
                    </div>
                    <div className="mode-selection-group">
                      <ModeSwitcher 
                        value={config.output_method} 
                        onToggle={toggleOutputMethod} 
                        options={[
                          { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter Mode: Simulates key presses' },
                          { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard Mode: Fast copy-paste' }
                        ]}
                      />
                      <div className="mode-description" key={`desc-${config.output_method}`}>
                        {config.output_method === 'Typewriter' 
                          ? 'Types directly into your active cursor.' 
                          : 'Copies results to your clipboard.'}
                      </div>
                    </div>
                  </div>
                  
                  <Card className="help-content">
                    <h3>How to Use Voquill</h3>
                    <ol className="instructions">
                      {config.transcription_mode === 'Local' ? (
                        modelStatus[config.local_model_size] ? (
                          <li>Local Whisper model is <strong>Ready</strong>.</li>
                        ) : (
                          <li>Download a <strong>Whisper model</strong> in Config.</li>
                        )
                      ) : (
                        <li>Enter your <strong>OpenAI API key</strong> in Config.</li>
                      )}
                      <li>Position cursor in any text field.</li>
                      <li>Hold <strong>{config.hotkey}</strong> and speak.</li>
                      <li>Release keys to transcribe and type.</li>
                    </ol>
                    </Card>

                    <div className="status-footer">
                      <div className="status-footer-links">
                        <Button variant="icon" className="github-link" onClick={() => open('https://github.com/jackbrumley/voquill')} title="GitHub Repository">
                          <IconBrandGithub size={18} />
                        </Button>
                        <Button variant="icon" className="github-link" onClick={() => open('https://voquill.org/donate')} title="Support the project">
                          <IconHeart size={18} color="#ff6b6b" fill="#ff6b6b" fillOpacity={0.2} />
                        </Button>
                      </div>
                      <div className="version-text">v{appVersion}</div>
                    </div>
                  </div>
                </div>

            )}

            {activeTab === 'config' && (
              <div className="tab-panel config-panel" key="config">
                <div className="tab-panel-content">
                  <CollapsibleSection title="Basic" isOpen={activeConfigSection === 'basic'} onToggle={() => setActiveConfigSection(activeConfigSection === 'basic' ? null : 'basic')}>
                    <ConfigField label="Transcription Method" description="Choose between cloud-based API or fully local processing.">
                      <ModeSwitcher 
                        value={config.transcription_mode} 
                        onToggle={(val) => updateConfig('transcription_mode', val)} 
                        options={[
                          { value: 'Local', label: 'Local', title: 'Run Whisper locally' },
                          { value: 'API', label: 'Cloud API', title: 'Use OpenAI API' }
                        ]}
                      />
                    </ConfigField>

                    <ConfigField label="Language" description="Hint the dialect or hard-set the output language.">
                      <div className="select-wrapper">
                        <select value={config.language} onChange={(e: any) => updateConfig('language', e.target.value)}>
                          <option value="auto">Automatic Detection</option>
                          <option value="en-AU">English (Australia)</option>
                          <option value="en-GB">English (United Kingdom)</option>
                          <option value="en-US">English (United States)</option>
                          <option value="fr">French</option>
                          <option value="es">Spanish</option>
                          <option value="de">German</option>
                          <option value="it">Italian</option>
                          <option value="pt">Portuguese</option>
                          <option value="nl">Dutch</option>
                          <option value="ja">Japanese</option>
                          <option value="zh">Chinese</option>
                        </select>
                      </div>
                    </ConfigField>

                    {config.transcription_mode === 'API' ? (
                      <>
                        <ConfigField label="API Key" description="Used to authenticate with the transcription service (OpenAI).">
                          <div className="input-with-button" style={{ display: 'flex', gap: '8px' }}>
                            <input type="text" value={config.openai_api_key} onChange={(e: any) => updateConfig('openai_api_key', e.target.value)} placeholder="sk-..." />
                            <Button onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? '...' : 'Test'}</Button>
                          </div>
                        </ConfigField>

                        <ConfigField label="API URL" description="The endpoint that processes audio (OpenAI or Local Whisper).">
                          <input type="url" value={config.api_url} onChange={(e: any) => updateConfig('api_url', e.target.value)} />
                        </ConfigField>
                        
                        <ConfigField label="API Model" description="The model name to use with the API provider.">
                          <input type="text" value={config.api_model} onChange={(e: any) => updateConfig('api_model', e.target.value)} />
                        </ConfigField>
                      </>
                    ) : (
                      <>
                        <ConfigField label="Local Engine" description="The core technology used to process your voice locally.">
                          <div className="select-wrapper">
                            <select value={config.local_engine} onChange={(e: any) => updateConfig('local_engine', e.target.value)}>
                              {availableEngines.map(engine => (
                                <option key={engine} value={engine}>{engine}</option>
                              ))}
                            </select>
                          </div>
                        </ConfigField>

                        <ConfigField label="Local Model" description="Choose the Whisper model size. Distil-Small is recommended for most users.">
                          <div className="select-wrapper">
                            {availableModels.length > 0 ? (
                              <>
                                <select value={config.local_model_size} onChange={(e: any) => updateConfig('local_model_size', e.target.value)}>
                                  {availableModels
                                    .filter(m => m.engine === config.local_engine)
                                    .map(m => (
                                      <option key={m.size} value={m.size}>
                                        {m.label} {m.recommended ? '(Recommended)' : ''} ({Math.round(m.file_size / 1024 / 1024)}MB)
                                      </option>
                                    ))
                                  }
                                </select>
                                <Button variant="ghost" className="icon-button" onClick={() => setShowModelGuide(true)} title="Model Guide">
                                  <IconInfoCircle size={20} />
                                </Button>
                                {!modelStatus[config.local_model_size] && (
                                  <Button size="sm" onClick={() => downloadModel(config.local_model_size)} disabled={isDownloading}>
                                    {isDownloading ? '...' : 'Download'}
                                  </Button>
                                )}
                              </>
                            ) : (
                              <div style={{ display: 'flex', gap: '8px', alignItems: 'center', width: '100%' }}>
                                <div style={{ fontSize: '12px', color: 'var(--color-text-dim)', flex: 1 }}>Loading models...</div>
                                <Button size="sm" onClick={loadModels}>Retry</Button>
                              </div>
                            )}
                          </div>
                          {availableModels.length > 0 && (
                            <div className="mode-description" style={{ textAlign: 'left', marginTop: '4px' }}>
                              {availableModels.find(m => m.size === config.local_model_size)?.description}
                            </div>
                          )}
                        </ConfigField>
                        {isDownloading && (
                          <div className="download-progress-container" style={{ marginTop: '-8px', marginBottom: '16px' }}>
                            <div className="volume-meter-container" style={{ height: '4px' }}>
                               <div className="volume-meter-bar" style={{ width: `${Math.min(downloadProgress, 100)}%`, background: 'var(--colors-accent-primary)' }}></div>
                            </div>
                            <div style={{ fontSize: '10px', color: 'var(--color-text-dim)', textAlign: 'right', marginTop: '2px' }}>Downloading model... {Math.round(downloadProgress)}%</div>
                          </div>
                        )}
                      </>
                    )}

                    <ConfigField label="Global Hotkey" description="Hold these keys to record, release to transcribe.">
                      <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                        <input 
                          type="text" 
                          value={isRecordingHotkey ? 'Press keys...' : config.hotkey} 
                          readOnly
                          onClick={() => {}}
                          placeholder="Configure using button"
                          className={`hotkey-input ${isRecordingHotkey ? 'recording' : ''}`}
                          style={{ opacity: portalVersion >= 1 ? 0.9 : 1, cursor: 'default' }}
                          title={portalVersion >= 1 ? 'Use Configure Hotkey to request binding through the system portal.' : ''}
                        />
                        <Button size="sm" variant="secondary" onClick={handleConfigureHotkey} disabled={isApplyingHotkey}>
                          Configure Hotkey
                        </Button>
                      </div>
                      {portalVersion >= 1 && (
                        <div style={{ fontSize: '11px', color: 'var(--colors-text-muted)', marginTop: '4px' }}>
                          Shortcut registration uses the Wayland GlobalShortcuts portal.
                          {portalDiagnostics?.active_trigger ? ` Active shortcut: ${portalDiagnostics.active_trigger}.` : ''}
                          {hotkeyBindingState?.bound ? ' Listener is active.' : ''}
                        </div>
                      )}
                    </ConfigField>

                    <ConfigField label="Always Copy to Clipboard" description="Automatically copies the transcription to your clipboard even when in Typewriter mode.">
                      <Switch checked={config.copy_on_typewriter} onChange={(checked) => updateConfig('copy_on_typewriter', checked)} label="Enabled" />
                    </ConfigField>
                  </CollapsibleSection>

                  <CollapsibleSection title="Audio" isOpen={activeConfigSection === 'audio'} onToggle={() => setActiveConfigSection(activeConfigSection === 'audio' ? null : 'audio')}>
                    <ConfigField label="Microphone" description="Choose the input device for recording your voice.">
                      <div className="select-wrapper">
                        <select value={config.audio_device || 'default'} onChange={(e: any) => updateConfig('audio_device', e.target.value)}>
                          {availableMics.map((mic: any) => <option key={mic.id} value={mic.id}>{mic.label}</option>)}
                        </select>
                        <Button variant="ghost" className="icon-button" onClick={loadMics} title="Refresh Devices">
                          <IconRefresh size={16} />
                        </Button>
                      </div>
                    </ConfigField>
                      
                    <ConfigField label={`Mic Sensitivity (${Math.round(config.input_sensitivity * 100)}%)`} description="Adjust the gain levels. Higher values pick up quieter sounds.">
                      <input type="range" min="0.1" max="2.0" step="0.05" value={config.input_sensitivity} onChange={(e: any) => updateConfig('input_sensitivity', parseFloat(e.target.value))} className="slider" />
                    </ConfigField>

                    <div className="mic-test-row">
                      <Button className="mic-test-button" disabled={micTestStatus === 'processing'} variant={micTestStatus !== 'idle' ? 'primary' : 'secondary'} onClick={() => { if (micTestStatus === 'idle') startMicTest(); else if (micTestStatus === 'recording') stopMicTest(); else if (micTestStatus === 'playing') stopMicPlayback(); }}>
                        {micTestStatus === 'idle' ? 'Test Microphone' : micTestStatus === 'recording' ? 'Stop & Play Back' : micTestStatus === 'playing' ? 'Stop Playback' : 'Processing...'}
                      </Button>
                      {micTestStatus === 'recording' && (
                        <div className="volume-meter-container">
                          <div className={`volume-meter-bar ${micVolume > 0.9 ? 'clipping' : micVolume > 0.7 ? 'warning' : ''}`} style={{ width: `${Math.min(micVolume * 100, 100)}%` }}></div>
                        </div>
                      )}
                    </div>
                  </CollapsibleSection>

                  <CollapsibleSection title="Typing" isOpen={activeConfigSection === 'typing'} onToggle={() => setActiveConfigSection(activeConfigSection === 'typing' ? null : 'typing')}>
                    <ConfigField label="Typing Speed (ms)" description="Delay between characters. Lower values are faster (1ms recommended).">
                      <input type="number" value={config.typing_speed_interval} onChange={(e: any) => updateConfig('typing_speed_interval', parseInt(e.target.value))} />
                    </ConfigField>

                    <ConfigField label="Key Press Duration (ms)" description="How long each key is held. Increase if characters are skipped.">
                      <input type="number" value={config.key_press_duration_ms} onChange={(e: any) => updateConfig('key_press_duration_ms', parseInt(e.target.value))} />
                    </ConfigField>
                  </CollapsibleSection>

                  <CollapsibleSection title="Advanced" isOpen={activeConfigSection === 'advanced'} onToggle={() => setActiveConfigSection(activeConfigSection === 'advanced' ? null : 'advanced')}>
                    <ConfigField label="Popup Position (px)" description="Vertical offset for the status overlay from the screen bottom.">
                      <input type="number" value={config.pixels_from_bottom} onChange={(e: any) => updateConfig('pixels_from_bottom', parseInt(e.target.value))} />
                    </ConfigField>

                    <ConfigField label="Debug Mode" description="Master switch for advanced diagnostic settings.">
                      <Switch checked={config.debug_mode} onChange={(checked) => updateConfig('debug_mode', checked)} label="Enable Debug Settings" />
                    </ConfigField>

                    <ConfigField label="Turbo Mode (GPU)" description="Uses your graphics card to speed up transcription. Recommended for 'Medium' models.">
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <Switch checked={config.enable_gpu} onChange={(checked) => updateConfig('enable_gpu', checked)} label="Enabled" />
                        <IconRocket size={20} style={{ color: config.enable_gpu ? '#f1c40f' : 'var(--colors-text-muted)', opacity: config.enable_gpu ? 1 : 0.5, transition: 'all 0.3s ease' }} />
                      </div>
                    </ConfigField>

                    {config.debug_mode && (
                      <ConfigField label="Recording Logs" description="Saves dictation recordings as WAV files to your app data folder to help analyze audio issues.">
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                          <Switch checked={config.enable_recording_logs} onChange={(checked) => updateConfig('enable_recording_logs', checked)} label="Enable Recording Logs" />
                          <Button size="sm" variant="ghost" onClick={openDebugFolder}>Open Folder</Button>
                        </div>
                      </ConfigField>
                    )}
                  </CollapsibleSection>
                </div>
              </div>
            )}

            {activeTab === 'history' && (
              <div className="tab-panel" key="history">
                <div className="tab-panel-padded">
                  <div className="history-list">
                    {history.length === 0 ? <Card className="empty-history"><p>No transcriptions yet.</p></Card> :
                      history.map((item) => (
                        <Card key={item.id} className="history-item">
                          <div className="history-text">{item.text}</div>
                          <Button variant="ghost" size="sm" className="copy-button" onClick={() => copyToClipboard(item.text)} title="Copy to clipboard">
                            <IconCopy size={14} />
                          </Button>
                          <div className="history-timestamp">{new Date(item.timestamp).toLocaleString()}</div>
                        </Card>
                      ))
                    }
                  </div>
                </div>
              </div>
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
