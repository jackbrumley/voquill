
import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';
import { IconBrandGithub } from '@tabler/icons-react';
import StatusIcon from './StatusIcon.tsx';
import { tokens, tokensToCssVars } from './design-tokens.ts';
import { Card } from './components/Card.tsx';
import { Button } from './components/Button.tsx';
import { ConfigField } from './components/ConfigField.tsx';
import { Switch } from './components/Switch.tsx';
import { CollapsibleSection } from './components/CollapsibleSection.tsx';
import { ModeSwitcher } from './components/ModeSwitcher.tsx';
import { ActionFooter } from './components/ActionFooter.tsx';
import './App.css';

interface Config {
  openai_api_key: string;
  api_url: string;
  api_model: string;
  transcription_mode: 'API' | 'Local';
  local_model_size: string;
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

function App() {
  const [config, setConfig] = useState<Config>({
    openai_api_key: '',
    api_url: 'https://api.openai.com/v1/audio/transcriptions',
    api_model: 'whisper-1',
    transcription_mode: 'Local',
    local_model_size: 'base',
    hotkey: 'ctrl+space',
    typing_speed_interval: 1,
    key_press_duration_ms: 2,
    pixels_from_bottom: 100,
    audio_device: 'default',
    debug_mode: false,
    enable_recording_logs: false,
    input_sensitivity: 1.0,
    output_method: 'Typewriter',
    copy_on_typewriter: false,
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
  const [availableModels, setAvailableModels] = useState<any[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<number>(0);
  const [isDownloading, setIsDownloading] = useState(false);
  const [modelStatus, setModelStatus] = useState<Record<string, boolean>>({});

  const logUI = (msg: string) => {
    if (!config.debug_mode && !msg.includes('Button clicked')) return;
    const timestamp = new Date().toLocaleTimeString();
    console.log(`[${timestamp}] ${msg}`);
    invoke('log_ui_event', { message: msg }).catch((err) => {
      console.error(`Failed to send log to backend: ${err}`);
    });
  };

  useEffect(() => {
    const cssVars = tokensToCssVars(tokens);
    const root = document.documentElement;
    Object.entries(cssVars).forEach(([key, value]) => {
      root.style.setProperty(key, value);
    });
  }, []);

  useEffect(() => {
    loadConfig();
    loadMics();
    loadHistory();
    loadModels();
    
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

    return () => {
      unlistenPressed.then((fn: any) => fn());
      unlistenReleased.then((fn: any) => fn());
      unlistenSetup.then((fn: any) => fn());
      unlistenStatus.then((fn: any) => fn());
      unlistenHistory.then((fn: any) => fn());
      unlistenMicTestStarted.then((fn: any) => fn());
      unlistenMicTestFinished.then((fn: any) => fn());
      unlistenMicVolume.then((fn: any) => fn());
      unlistenDownloadProgress.then((fn: any) => fn());
    };
  }, []);

  useEffect(() => {
    if (config.transcription_mode === 'Local' && availableModels.length === 0) {
      loadModels();
    }
  }, [config.transcription_mode]);

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

  const updateConfig = (key: keyof Config, value: any, shouldPersist = false) => {
    setConfig(prev => {
      const newConfig = { ...prev, [key]: value };
      if (shouldPersist) {
        persistConfig(newConfig);
      }
      return newConfig;
    });
  };

  const saveConfig = async () => {
    logUI('🖱️ Button clicked: Save Configuration');
    await persistConfig(config);
    showToast('Configuration saved!', 'success');
  };

  const toggleOutputMethod = (method: 'Typewriter' | 'Clipboard') => {
    logUI(`🖱️ Output Method changed to: ${method}`);
    updateConfig('output_method', method, true);
  };

  const startMicTest = async () => {
    logUI('🖱️ Button clicked: Check Microphone (Start)');
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
    const id = Date.now();
    setToasts(prev => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, 3000);
  };

  const handleClose = async () => {
    await getCurrentWindow().hide();
  };

  const handleMinimize = async () => {
    await getCurrentWindow().minimize();
  };

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
                <ModeSwitcher 
                  value={config.output_method} 
                  onToggle={toggleOutputMethod} 
                  options={[
                    { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter Mode: Simulates key presses' },
                    { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard Mode: Fast copy-paste' }
                  ]}
                />
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
                <span className="version-text">v{appVersion}</span>
                <button 
                  className="github-link" 
                  onClick={() => open('https://github.com/jackbrumley/voquill')}
                  title="View on GitHub"
                >
                  <IconBrandGithub size={16} />
                </button>
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
                    onToggle={(val) => updateConfig('transcription_mode', val, true)} 
                    options={[
                      { value: 'Local', label: 'Local', title: 'Run Whisper locally' },
                      { value: 'API', label: 'Cloud API', title: 'Use OpenAI API' }
                    ]}
                  />
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
                    <ConfigField label="Local Model" description="Choose the Whisper model size. Larger models are more accurate but slower.">
                      <div className="select-wrapper">
                        {availableModels.length > 0 ? (
                          <>
                            <select value={config.local_model_size} onChange={(e: any) => updateConfig('local_model_size', e.target.value, true)}>
                              {availableModels.map(m => (
                                <option key={m.size} value={m.size}>{m.size.charAt(0).toUpperCase() + m.size.slice(1)} ({Math.round(m.file_size / 1024 / 1024)}MB)</option>
                              ))}
                            </select>
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
                    </ConfigField>
                    {isDownloading && (
                      <div className="download-progress-container" style={{ marginTop: '-8px', marginBottom: '16px' }}>
                        <div className="volume-meter-container" style={{ height: '4px' }}>
                           <div className="volume-meter-bar" style={{ width: `${downloadProgress}%`, background: 'var(--color-primary)' }}></div>
                        </div>
                        <div style={{ fontSize: '10px', color: 'var(--color-text-dim)', textAlign: 'right', marginTop: '2px' }}>Downloading model... {Math.round(downloadProgress)}%</div>
                      </div>
                    )}
                  </>
                )}
              </CollapsibleSection>

              <CollapsibleSection title="Audio" isOpen={activeConfigSection === 'audio'} onToggle={() => setActiveConfigSection(activeConfigSection === 'audio' ? null : 'audio')}>
                <ConfigField label="Microphone" description="Choose the input device for recording your voice.">
                  <div className="select-wrapper">
                    <select value={config.audio_device || 'default'} onChange={(e: any) => updateConfig('audio_device', e.target.value, true)}>
                      {availableMics.map((mic: any) => <option key={mic.id} value={mic.id}>{mic.label}</option>)}
                    </select>
                    <Button variant="ghost" className="icon-button" onClick={loadMics} title="Refresh Devices">
                      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" strokeWidth="2" stroke="currentColor" fill="none" strokeLinecap="round" strokeLinejoin="round">
                        <path stroke="none" d="M0 0h24v24H0z" fill="none"/><path d="M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4" /><path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4" />
                      </svg>
                    </Button>
                  </div>
                </ConfigField>
                  
                <ConfigField label={`Mic Sensitivity (${Math.round(config.input_sensitivity * 100)}%)`} description="Adjust the gain levels. Higher values pick up quieter sounds.">
                  <input type="range" min="0.1" max="2.0" step="0.05" value={config.input_sensitivity} onChange={(e: any) => updateConfig('input_sensitivity', parseFloat(e.target.value))} className="slider" />
                </ConfigField>

                <div className="mic-test-row">
                  <Button className="mic-test-button" disabled={micTestStatus === 'processing'} variant={micTestStatus !== 'idle' ? 'primary' : 'secondary'} onClick={() => { if (micTestStatus === 'idle') startMicTest(); else if (micTestStatus === 'recording') stopMicTest(); else if (micTestStatus === 'playing') stopMicPlayback(); }}>
                    {micTestStatus === 'idle' ? 'Check Microphone' : micTestStatus === 'recording' ? 'Stop & Play Back' : micTestStatus === 'playing' ? 'Stop Playback' : 'Processing...'}
                  </Button>
                  {micTestStatus === 'recording' && (
                    <div className="volume-meter-container">
                      <div className={`volume-meter-bar ${micVolume > 0.9 ? 'clipping' : micVolume > 0.7 ? 'warning' : ''}`} style={{ width: `${Math.min(micVolume * 100, 100)}%` }}></div>
                    </div>
                  )}
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Typing" isOpen={activeConfigSection === 'typing'} onToggle={() => setActiveConfigSection(activeConfigSection === 'typing' ? null : 'typing')}>
                <ConfigField label="Always Copy to Clipboard" description="Automatically copies the transcription to your clipboard even when in Typewriter mode.">
                  <Switch checked={config.copy_on_typewriter} onChange={(checked) => updateConfig('copy_on_typewriter', checked, true)} label="Enabled" />
                </ConfigField>

                <ConfigField label="Global Hotkey" description="Hold these keys to record, release to transcribe.">
                  <input type="text" value={config.hotkey} onChange={(e: any) => updateConfig('hotkey', e.target.value)} />
                </ConfigField>

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
                  <Switch checked={config.debug_mode} onChange={(checked) => updateConfig('debug_mode', checked, true)} label="Enable Debug Settings" />
                </ConfigField>

                {config.debug_mode && (
                  <ConfigField label="Recording Logs" description="Saves dictation recordings as WAV files to your app data folder to help analyze audio issues.">
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                      <Switch checked={config.enable_recording_logs} onChange={(checked) => updateConfig('enable_recording_logs', checked, true)} label="Enable Recording Logs" />
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
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                        </svg>
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

      {activeTab === 'config' && (
        <ActionFooter>
          <Button variant="primary" className="sticky-footer-button" onClick={saveConfig}>Save Configuration</Button>
        </ActionFooter>
      )}

      {activeTab === 'history' && (
        <ActionFooter>
          <Button variant="danger" className="sticky-footer-button" onClick={clearHistory}>Clear History</Button>
        </ActionFooter>
      )}

      <div className="toast-container">
        {toasts.map(toast => (
          <div key={toast.id} className={`toast ${toast.type}`} onClick={() => setToasts(prev => prev.filter(t => t.id !== toast.id))}>
            <span className="toast-dot"></span>
            <span className="toast-message">{toast.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
