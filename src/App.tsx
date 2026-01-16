import { useState, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import StatusIcon from './StatusIcon.tsx';
import './App.css';

interface Config {
  openai_api_key: string;
  api_url: string;
  hotkey: string;
  typing_speed_interval: number;
  pixels_from_bottom: number;
  audio_device: string | null;
  debug_mode: boolean;
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
    typing_speed_interval: 0.01,
    pixels_from_bottom: 100,
    audio_device: 'default',
    debug_mode: false,
    input_sensitivity: 1.0,
  });
  
  const [activeTab, setActiveTab] = useState<'status' | 'history' | 'config'>('status');
  const [isTestingApi, setIsTestingApi] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [availableMics, setAvailableMics] = useState<AudioDevice[]>([]);
  const [micTestStatus, setMicTestStatus] = useState<'idle' | 'recording' | 'playing'>('idle');
  const [micVolume, setMicVolume] = useState<number>(0);

  const logUI = (msg: string) => {
    const timestamp = new Date().toLocaleTimeString();
    console.log(`[${timestamp}] ${msg}`);
    invoke('log_ui_event', { message: msg }).catch(() => {});
  };

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
    
    // Listen for mic test playback events
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

  const updateConfig = (key: keyof Config, value: any) => {
    setConfig(prev => ({ ...prev, [key]: value }));
  };

  const saveConfig = async () => {
    logUI('🖱️ Button clicked: Save Configuration');
    try {
      const configToSave = {
        openai_api_key: config.openai_api_key || 'your_api_key_here',
        api_url: config.api_url,
        hotkey: config.hotkey,
        typing_speed_interval: config.typing_speed_interval / 1000,
        pixels_from_bottom: config.pixels_from_bottom,
        audio_device: config.audio_device,
        debug_mode: config.debug_mode,
        input_sensitivity: config.input_sensitivity,
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
          <button className="title-bar-button minimize" onClick={handleMinimize}>─</button>
          <button className="title-bar-button close" onClick={handleClose}>✕</button>
        </div>
      </div>

      <div className="tab-nav">
        <button className={`tab ${activeTab === 'status' ? 'active' : ''}`} onClick={() => setActiveTab('status')}>Status</button>
        <button className={`tab ${activeTab === 'history' ? 'active' : ''}`} onClick={() => setActiveTab('history')}>History</button>
        <button className={`tab ${activeTab === 'config' ? 'active' : ''}`} onClick={() => setActiveTab('config')}>Config</button>
      </div>

      <div className="tab-content">
        {activeTab === 'status' && (
          <div className="tab-panel">
            <div className="status-display">
              <StatusIcon status={currentStatus} />
              <div className="status-text-app">{currentStatus}</div>
            </div>
            <div className="record-section">
              <div className="help-text">Hotkey: <strong>{config.hotkey}</strong></div>
            </div>
            <div className="help-content">
              <h3>How to Use Voquill</h3>
              <ol className="instructions">
                <li>Enter your OpenAI API key in the Config tab</li>
                <li>Position cursor in any text field</li>
                <li>Hold <strong>{config.hotkey}</strong> and speak</li>
                <li>Release keys to transcribe and type</li>
              </ol>
            </div>
          </div>
        )}

        {activeTab === 'config' && (
          <div className="tab-panel">
            <div className="form-group">
              <label>API Key:</label>
              <div className="input-with-button">
                <input type="password" value={config.openai_api_key} onChange={(e: any) => updateConfig('openai_api_key', e.target.value)} placeholder="sk-..." />
                <button className="button" onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? 'Testing...' : 'Test Key'}</button>
              </div>
            </div>

            <div className="form-group">
              <label>Microphone:</label>
              <div className="select-wrapper">
                <select value={config.audio_device || 'default'} onChange={(e: any) => updateConfig('audio_device', e.target.value)}>
                  {availableMics.map((mic: any) => <option key={mic.id} value={mic.id}>{mic.label}</option>)}
                </select>
                <button className="button small icon-button" onClick={loadMics} title="Refresh Devices">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" strokeWidth="2" stroke="currentColor" fill="none" strokeLinecap="round" strokeLinejoin="round">
                    <path stroke="none" d="M0 0h24v24H0z" fill="none"/>
                    <path d="M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4" />
                    <path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4" />
                  </svg>
                </button>
              </div>
              
              <div className="form-group sensitivity-section">
                <div className="label-with-value">
                  <label>Mic Sensitivity:</label>
                  <span className="value-badge">{Math.round(config.input_sensitivity * 100)}%</span>
                </div>
                <input 
                  type="range" 
                  min="0.1" 
                  max="2.0" 
                  step="0.05" 
                  value={config.input_sensitivity} 
                  onChange={(e: any) => updateConfig('input_sensitivity', parseFloat(e.target.value))}
                  className="slider"
                />
              </div>

              <div className="mic-test-row">
                <div className="mic-test-controls">
                  <button 
                    className={`button mic-test-button ${micTestStatus !== 'idle' ? 'active' : ''}`} 
                    onClick={() => {
                      if (micTestStatus === 'idle') startMicTest();
                      else if (micTestStatus === 'recording') stopMicTest();
                      else if (micTestStatus === 'playing') stopMicPlayback();
                    }}
                  >
                    {micTestStatus === 'idle' && 'Check Microphone'}
                    {micTestStatus === 'recording' && 'Stop & Play Back'}
                    {micTestStatus === 'playing' && 'Stop Playback'}
                  </button>
                  {micTestStatus === 'recording' && (
                    <div className="volume-meter-container">
                      <div 
                        className={`volume-meter-bar ${micVolume > 0.9 ? 'clipping' : micVolume > 0.7 ? 'warning' : ''}`} 
                        style={{ width: `${Math.min(micVolume * 100, 100)}%` }}
                      ></div>
                    </div>
                  )}
                </div>
              </div>
            </div>

            <div className="form-group debug-section">
              <div className="debug-toggle">
                <label className="checkbox-label">
                  <input 
                    type="checkbox" 
                    checked={config.debug_mode} 
                    onChange={(e: any) => updateConfig('debug_mode', e.target.checked)} 
                  />
                  Enable Debug Mode
                </label>
                <button className="button small" onClick={openDebugFolder}>Open Debug Folder</button>
              </div>
              <p className="debug-help-text">
                When enabled, Voquill saves your dictation recordings to disk to help troubleshoot audio quality. 
                Files are stored in your app data folder.
              </p>
            </div>

            <hr className="config-divider" />

            <div className="form-group">
              <label>API URL:</label>
              <input type="url" value={config.api_url} onChange={(e: any) => updateConfig('api_url', e.target.value)} />
            </div>

            <div className="form-group">
              <label>Global Hotkey:</label>
              <input type="text" value={config.hotkey} onChange={(e: any) => updateConfig('hotkey', e.target.value)} />
            </div>

            <div className="form-group">
              <label>Typing Speed (ms):</label>
              <input type="number" value={config.typing_speed_interval} onChange={(e: any) => updateConfig('typing_speed_interval', parseInt(e.target.value))} />
            </div>

            <div className="form-group">
              <label>Popup Position (px from bottom):</label>
              <input type="number" value={config.pixels_from_bottom} onChange={(e: any) => updateConfig('pixels_from_bottom', parseInt(e.target.value))} />
            </div>

            <div className="form-actions-bottom">
              <button className="button primary save-button" onClick={saveConfig}>Save Configuration</button>
            </div>
          </div>
        )}

        {activeTab === 'history' && (
          <div className="tab-panel">
            <div className="history-header">
              <button className="button clear-history-button" onClick={clearHistory}>Clear History</button>
            </div>
            <div className="history-list">
              {history.length === 0 ? <div className="empty-history"><p>No transcriptions yet.</p></div> :
                history.map((item) => (
                  <div key={item.id} className="history-item">
                    <div className="history-time">{new Date(item.timestamp).toLocaleString()}</div>
                    <div className="history-text">{item.text}</div>
                  </div>
                ))
              }
            </div>
          </div>
        )}
      </div>

      <div className="toast-container">
        {toasts.map(toast => (
          <div key={toast.id} className={`toast ${toast.type}`}>
            {toast.message}
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
