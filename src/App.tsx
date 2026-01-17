
import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import StatusIcon from './StatusIcon.tsx';
import { tokens, tokensToCssVars } from './design-tokens.ts';
import { Card } from './components/Card.tsx';
import { Button } from './components/Button.tsx';
import { ConfigField } from './components/ConfigField.tsx';
import { Switch } from './components/Switch.tsx';
import './App.css';

interface Config {
  openai_api_key: string;
  api_url: string;
  hotkey: string;
  typing_speed_interval: number;
  key_press_duration_ms: number;
  pixels_from_bottom: number;
  audio_device: string | null;
  debug_mode: boolean;
  enable_recording_logs: boolean;
  input_sensitivity: number;
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
    hotkey: 'ctrl+space',
    typing_speed_interval: 0.001,
    key_press_duration_ms: 2,
    pixels_from_bottom: 100,
    audio_device: 'default',
    debug_mode: false,
    enable_recording_logs: false,
    input_sensitivity: 1.0,
  });
  
  const [activeTab, setActiveTab] = useState<'status' | 'history' | 'config'>('status');
  const [isTestingApi, setIsTestingApi] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [availableMics, setAvailableMics] = useState<AudioDevice[]>([]);
  const [micTestStatus, setMicTestStatus] = useState<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const [micVolume, setMicVolume] = useState<number>(0);

  const logUI = (msg: string) => {
    const timestamp = new Date().toLocaleTimeString();
    console.log(`[${timestamp}] ${msg}`);
    invoke('log_ui_event', { message: msg }).catch((err) => {
      console.error(`Failed to send log to backend: ${err}`);
    });
  };

  // Inject design tokens into CSS variables
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

    const unlistenPressed = listen('hotkey-pressed', () => {
      logUI('📥 Event received: hotkey-pressed');
      setCurrentStatus('Recording');
    });

    const unlistenReleased = listen('hotkey-released', () => {
      logUI('📥 Event received: hotkey-released');
      setCurrentStatus('Transcribing');
    });

    const unlistenSetup = listen<string>('setup-status', (event) => {
      logUI(`📥 Event received: setup-status (${event.payload})`);
      if (event.payload === 'configuring-system') {
        showToast('Configuring system permissions...', 'info');
      } else if (event.payload === 'restart-required') {
        showToast('Permissions updated! Please restart your session (logout/login).', 'success');
      } else if (event.payload === 'setup-failed') {
        showToast('System configuration failed. Please check logs.', 'error');
      }
    });

    const unlistenHotkeyError = listen<string>('hotkey-error', (event) => {
      logUI(`📥 Event received: hotkey-error (${event.payload})`);
      showToast(`Hotkey Error: ${event.payload}`, 'error');
    });

    const unlistenAudioError = listen<string>('audio-error', (event) => {
      logUI(`📥 Event received: audio-error (${event.payload})`);
      if (event.payload === 'portal-denied') {
        showToast('Microphone access denied via system portal.', 'error');
      } else {
        showToast(`Audio Error: ${event.payload}`, 'error');
      }
    });

    const unlistenStatus = listen<string>('status-update', (event) => {
      logUI(`📥 Event received: status-update (${event.payload})`);
      setCurrentStatus(event.payload);
    });

    const unlistenHistory = listen('history-updated', () => {
      logUI('📥 Event received: history-updated');
      loadHistory();
    });
    
    const unlistenMicTestStarted = listen('mic-test-playback-started', () => {
      logUI('📥 Event received: mic-test-playback-started');
      setMicTestStatus('playing');
    });

    const unlistenMicTestFinished = listen('mic-test-playback-finished', () => {
      logUI('📥 Event received: mic-test-playback-finished');
      setMicTestStatus('idle');
      setMicVolume(0);
    });

    const unlistenMicVolume = listen<number>('mic-test-volume', (event: any) => {
      setMicVolume(event.payload as number);
    });

    return () => {
      unlistenPressed.then((fn: any) => fn());
      unlistenReleased.then((fn: any) => fn());
      unlistenSetup.then((fn: any) => fn());
      unlistenHotkeyError.then((fn: any) => fn());
      unlistenAudioError.then((fn: any) => fn());
      unlistenStatus.then((fn: any) => fn());
      unlistenHistory.then((fn: any) => fn());
      unlistenMicTestStarted.then((fn: any) => fn());
      unlistenMicTestFinished.then((fn: any) => fn());
      unlistenMicVolume.then((fn: any) => fn());
    };
  }, []);

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
    logUI('🖱️ Button clicked: Refresh Devices');
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

  const updateConfig = (key: keyof Config, value: any) => {
    setConfig(prev => ({ ...prev, [key]: value }));
  };

  const saveConfig = async () => {
    logUI('🖱️ Button clicked: Save Configuration');
    try {
      const configToSave = {
        ...config,
        typing_speed_interval: config.typing_speed_interval / 1000,
        openai_api_key: config.openai_api_key || 'your_api_key_here',
      };
      
      await invoke('save_config', { newConfig: configToSave });
      showToast('Configuration saved!', 'success');
    } catch (error) {
      showToast(`Failed to save configuration: ${error}`, 'error');
    }
  };

  const startMicTest = async () => {
    logUI('🖱️ Button clicked: Check Microphone (Start)');
    try {
      setMicTestStatus('recording');
      await invoke('start_mic_test');
    } catch (error) {
      logUI(`❌ start_mic_test failed: ${error}`);
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
      logUI(`❌ stop_mic_test failed: ${error}`);
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
      logUI(`❌ stop_mic_playback failed: ${error}`);
      showToast(`Failed to stop playback: ${error}`, 'error');
    }
  };

  const openDebugFolder = async () => {
    logUI('🖱️ Button clicked: Open Debug Folder');
    try {
      await invoke('open_debug_folder');
    } catch (error) {
      showToast(`Failed to open debug folder: ${error}`, 'error');
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
    logUI('🖱️ Window Control: Close (Hide)');
    await getCurrentWindow().hide();
  };

  const handleMinimize = async () => {
    logUI('🖱️ Window Control: Minimize');
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
        <button className={`tab ${activeTab === 'status' ? 'active' : ''}`} onClick={() => { logUI('🖱️ Tab switched: Status'); setActiveTab('status'); }}>Status</button>
        <button className={`tab ${activeTab === 'history' ? 'active' : ''}`} onClick={() => { logUI('🖱️ Tab switched: History'); setActiveTab('history'); }}>History</button>
        <button className={`tab ${activeTab === 'config' ? 'active' : ''}`} onClick={() => { logUI('🖱️ Tab switched: Config'); setActiveTab('config'); }}>Config</button>
      </div>

      <div className="tab-content">
        {activeTab === 'status' && (
          <div className="tab-panel">
            <div className="status-display">
              <StatusIcon status={currentStatus} />
              <div className="status-text-app">{currentStatus}</div>
            </div>
            
            <Card variant="primary" className="help-content">
              <h3>How to Use Voquill</h3>
              <ol className="instructions">
                <li>Enter your <strong>OpenAI API key</strong> in Config.</li>
                <li>Position cursor in any text field.</li>
                <li>Hold <strong>{config.hotkey}</strong> and speak.</li>
                <li>Release keys to transcribe and type.</li>
              </ol>
            </Card>
          </div>
        )}

        {activeTab === 'config' && (
          <div className="tab-panel">
            <ConfigField 
              label="API Key" 
              description="Used to authenticate with the transcription service (OpenAI)."
            >
              <div className="input-with-button" style={{ display: 'flex', gap: '8px' }}>
                <input type="password" value={config.openai_api_key} onChange={(e: any) => updateConfig('openai_api_key', e.target.value)} placeholder="sk-..." />
                <Button onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? '...' : 'Test'}</Button>
              </div>
            </ConfigField>

            <ConfigField 
              label="Microphone" 
              description="Choose the input device for recording your voice."
            >
              <div className="select-wrapper">
                <select value={config.audio_device || 'default'} onChange={(e: any) => updateConfig('audio_device', e.target.value)}>
                  {availableMics.map((mic: any) => <option key={mic.id} value={mic.id}>{mic.label}</option>)}
                </select>
                <Button variant="ghost" className="icon-button" onClick={loadMics} title="Refresh Devices">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" strokeWidth="2" stroke="currentColor" fill="none" strokeLinecap="round" strokeLinejoin="round">
                    <path stroke="none" d="M0 0h24v24H0z" fill="none"/>
                    <path d="M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4" />
                    <path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4" />
                  </svg>
                </Button>
              </div>
            </ConfigField>
              
            <ConfigField 
              label={`Mic Sensitivity (${Math.round(config.input_sensitivity * 100)}%)`}
              description="Adjust the gain levels. Higher values pick up quieter sounds."
            >
              <input 
                type="range" min="0.1" max="2.0" step="0.05" 
                value={config.input_sensitivity} 
                onChange={(e: any) => updateConfig('input_sensitivity', parseFloat(e.target.value))}
                className="slider"
              />
            </ConfigField>

            <Card className="mic-test-row">
              <Button 
                className={`mic-test-button ${micTestStatus !== 'idle' ? 'active' : ''}`} 
                disabled={micTestStatus === 'processing'}
                onClick={() => {
                  if (micTestStatus === 'idle') startMicTest();
                  else if (micTestStatus === 'recording') stopMicTest();
                  else if (micTestStatus === 'playing') stopMicPlayback();
                }}
              >
                {micTestStatus === 'idle' ? 'Check Microphone' : 
                 micTestStatus === 'recording' ? 'Stop & Play Back' :
                 micTestStatus === 'playing' ? 'Stop Playback' :
                 'Processing...'}
              </Button>
              {micTestStatus === 'recording' ? (
                <div className="volume-meter-container">
                  <div 
                    className={`volume-meter-bar ${micVolume > 0.9 ? 'clipping' : micVolume > 0.7 ? 'warning' : ''}`} 
                    style={{ width: `${Math.min(micVolume * 100, 100)}%` }}
                  ></div>
                </div>
              ) : null}
            </Card>

            <hr className="config-divider" />

            <ConfigField label="API URL" description="The endpoint that processes audio (OpenAI or Local Whisper).">
              <input type="url" value={config.api_url} onChange={(e: any) => updateConfig('api_url', e.target.value)} />
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

            <ConfigField label="Popup Position (px)" description="Vertical offset for the status overlay from the screen bottom.">
              <input type="number" value={config.pixels_from_bottom} onChange={(e: any) => updateConfig('pixels_from_bottom', parseInt(e.target.value))} />
            </ConfigField>

            <hr className="config-divider" />

            <ConfigField 
              label="Debug Mode" 
              description="Master switch for advanced diagnostic settings."
            >
              <Switch 
                checked={config.debug_mode} 
                onChange={(checked) => updateConfig('debug_mode', checked)} 
                label="Enable Debug Settings"
              />
            </ConfigField>

            {config.debug_mode ? (
              <ConfigField 
                label="Recording Logs" 
                description="Saves dictation recordings as WAV files to your app data folder to help analyze audio issues."
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <Switch 
                    checked={config.enable_recording_logs} 
                    onChange={(checked) => updateConfig('enable_recording_logs', checked)} 
                    label="Enable Recording Logs"
                  />
                  <Button size="sm" variant="ghost" onClick={openDebugFolder}>Open Folder</Button>
                </div>
              </ConfigField>
            ) : null}

            <div className="form-actions-bottom">
              <Button variant="primary" className="save-button" onClick={saveConfig}>Save Configuration</Button>
            </div>
          </div>
        )}

        {activeTab === 'history' && (
          <div className="tab-panel">
            <div className="history-header">
              <Button variant="danger" size="sm" onClick={clearHistory}>Clear History</Button>
            </div>
            <div className="history-list">
              {history.length === 0 ? <Card className="empty-history"><p>No transcriptions yet.</p></Card> :
                history.map((item) => (
                  <Card key={item.id} className="history-item">
                    <div className="history-text">{item.text}</div>
                    <Button variant="ghost" size="sm" className="copy-button" onClick={() => copyToClipboard(item.text)} title="Copy to clipboard">
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                      </svg>
                    </Button>
                    <div className="history-timestamp">{new Date(item.timestamp).toLocaleString()}</div>
                  </Card>
                ))
              }
            </div>
          </div>
        )}
      </div>

      <div className="toast-container">
        {toasts.map(toast => (
          <div key={toast.id} className={`toast ${toast.type}`} onClick={() => setToasts(prev => prev.filter(t => t.id !== toast.id))}>
            {toast.message}
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
