import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import StatusIcon from './StatusIcon';
import './App.css';

interface Config {
  openai_api_key: string;
  api_url: string;
  hotkey: string;
  typing_speed_interval: number;
  pixels_from_bottom: number;
  audio_device: string | null;
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
    audio_device: null,
  });
  
  const [activeTab, setActiveTab] = useState<'status' | 'history' | 'config'>('status');
  const [isTestingApi, setIsTestingApi] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [availableMics, setAvailableMics] = useState<AudioDevice[]>([]);

  // Apply status class to body for animations (like overlay)
  useEffect(() => {
    const statusClass = getStatusClass(currentStatus);
    document.body.className = statusClass;
    
    return () => {
      document.body.className = '';
    };
  }, [currentStatus]);

  // Load configuration, history, and mics on startup
  useEffect(() => {
    loadConfig();
    loadHistory();
    loadMics();
    
    // Listen for hotkey events
    const unlistenPressed = listen('hotkey-pressed', async () => {
      console.log('🎤 Hotkey pressed - backend handling start');
      setCurrentStatus('Recording');
    });
    
    const unlistenReleased = listen('hotkey-released', async () => {
      console.log('⏹️ Hotkey released - backend handling stop');
    });

    // Listen for system setup status
    const unlistenSetup = listen('setup-status', (event: any) => {
      const status = event.payload as string;
      if (status === 'configuring-system') {
        showToast('ℹ️ System Setup: Configuring permissions for typing and audio (Password required)...', 'info');
      } else if (status === 'setup-success') {
        showToast('✅ System Ready: Typing and hotkeys enabled.', 'success');
      } else if (status === 'restart-required') {
        showToast('⚠️ Permissions updated: Please log out and back in to enable typing and audio.', 'info');
      } else if (status.startsWith('setup-failed')) {
        showToast(`❌ System Setup Failed: ${status.split(':')[1] || 'Unknown error'}`, 'error');
      }
    });

    // Listen for hotkey errors
    const unlistenHotkeyError = listen('hotkey-error', (event: any) => {
      const error = event.payload as string;
      if (error.startsWith('conflict')) {
        const key = error.split(':')[1] || 'shortcut';
        showToast(`⚠️ Hotkey Conflict: '${key}' is used by another app. Please change it in Config.`, 'error');
      } else {
        showToast(`❌ Hotkey Error: ${error.split(':')[1] || 'Failed to register'}`, 'error');
      }
    });

    // Listen for audio errors
    const unlistenAudioError = listen('audio-error', (event: any) => {
      const error = event.payload as string;
      if (error === 'device-busy') {
        showToast('❌ Microphone Busy: Another app is using the mic exclusively. Please close it and try again.', 'error');
      } else if (error === 'portal-denied') {
        showToast('⚠️ Microphone Access Denied: Please allow access in system settings to record.', 'error');
      } else {
        showToast(`❌ Microphone Error: ${error.split(':')[1] || 'Failed to access mic'}`, 'error');
      }
    });

    // Listen for status updates from backend
    const unlistenStatus = listen('status-update', (event: any) => {
      const status = event.payload as string;
      console.log('📊 Status update:', status);
      setCurrentStatus(status);
    });

    // Listen for history updates from backend
    const unlistenHistory = listen('history-updated', () => {
      console.log('📚 History updated - reloading');
      loadHistory();
    });

    return () => {
      unlistenPressed.then((fn: any) => fn());
      unlistenReleased.then((fn: any) => fn());
      unlistenSetup.then((fn: any) => fn());
      unlistenHotkeyError.then((fn: any) => fn());
      unlistenAudioError.then((fn: any) => fn());
      unlistenStatus.then((fn: any) => fn());
      unlistenHistory.then((fn: any) => fn());
    };
  }, []);

  // Load history when switching to history tab
  useEffect(() => {
    if (activeTab === 'history') {
      loadHistory();
    }
  }, [activeTab]);

  const showToast = (message: string, type: 'success' | 'error' | 'info') => {
    const id = Date.now();
    setToasts((prev: Toast[]) => [...prev, { id, message, type }]);
    
    setTimeout(() => {
      setToasts((prev: Toast[]) => prev.filter((toast: Toast) => toast.id !== id));
    }, 5000);
  };

  const removeToast = (id: number) => {
    setToasts((prev: Toast[]) => prev.filter((toast: Toast) => toast.id !== id));
  };

  const loadMics = async () => {
    try {
      const mics = await invoke<AudioDevice[]>('get_audio_devices');
      setAvailableMics(mics);
    } catch (error) {
      console.error('Failed to load microphones:', error);
    }
  };

  const loadConfig = async () => {
    try {
      const loadedConfig = await invoke<Config>('get_config');
      setConfig({
        ...loadedConfig,
        openai_api_key: loadedConfig.openai_api_key === 'your_api_key_here' ? '' : loadedConfig.openai_api_key,
        typing_speed_interval: loadedConfig.typing_speed_interval * 1000, // Convert to ms
      });
    } catch (error) {
      showToast('Failed to load configuration', 'error');
    }
  };

  const loadHistory = async () => {
    try {
      const historyData = await invoke<{ items: HistoryItem[] }>('get_history');
      setHistory(historyData.items);
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  };

  const clearHistory = async () => {
    try {
      await invoke('clear_history');
      setHistory([]);
      showToast('History cleared successfully', 'success');
    } catch (error) {
      showToast('Failed to clear history', 'error');
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast('Copied to clipboard!', 'success');
    } catch (error) {
      showToast('Failed to copy to clipboard', 'error');
    }
  };

  const saveConfig = async () => {
    try {
      const configToSave = {
        openai_api_key: config.openai_api_key || 'your_api_key_here',
        api_url: config.api_url,
        hotkey: config.hotkey,
        typing_speed_interval: config.typing_speed_interval / 1000,
        pixels_from_bottom: config.pixels_from_bottom,
        audio_device: config.audio_device,
      };
      
      await invoke('save_config', { newConfig: configToSave });
      showToast('Configuration saved!', 'success');
    } catch (error) {
      showToast(`Failed to save configuration: ${error}`, 'error');
    }
  };

  const testApiKey = async () => {
    if (!config.openai_api_key) {
      showToast('Please enter an API key first', 'error');
      return;
    }
    setIsTestingApi(true);
    try {
      const isValid = await invoke<boolean>('test_api_key', { 
        apiKey: config.openai_api_key,
        apiUrl: config.api_url 
      });
      if (isValid) {
        showToast('API key is valid!', 'success');
      } else {
        showToast('API key is invalid or has no credits', 'error');
      }
    } catch (error) {
      showToast(`Failed to test API key: ${error}`, 'error');
    } finally {
      setIsTestingApi(false);
    }
  };

  const updateConfig = (field: keyof Config, value: any) => {
    if (field === 'audio_device') {
      const deviceLabel = availableMics.find((m: AudioDevice) => m.id === value)?.label || 'System Default';
      console.log(`🎤 Audio device changed to: ${deviceLabel} (ID: ${value})`);
    }
    setConfig((prev: Config) => ({ ...prev, [field]: value }));
  };

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'Ready': return 'status-ready';
      case 'Recording': return 'status-recording';
      case 'Converting audio':
      case 'Transcribing': return 'status-transcribing';
      case 'Typing': return 'status-typing';
      default: return '';
    }
  };

  const formatTimestamp = (timestamp: string) => {
    try {
      return new Date(timestamp).toLocaleString();
    } catch (error) {
      return timestamp;
    }
  };

  const handleMinimize = async () => {
    await getCurrentWindow().minimize();
  };

  const handleClose = async () => {
    await getCurrentWindow().close();
  };

  const handleTitleBarMouseDown = async (e: React.MouseEvent) => {
    if (e.button === 0 && !(e.target as HTMLElement).closest('.title-bar-button')) {
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
              <StatusIcon status={currentStatus} className="app-status-icon" />
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
            <div className="form-actions">
              <button className="button primary" onClick={saveConfig}>Save Configuration</button>
              <button className="button" onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? 'Testing...' : 'Test API Key'}</button>
            </div>

            <div className="form-group">
              <label>API Key:</label>
              <input type="password" value={config.openai_api_key} onChange={(e) => updateConfig('openai_api_key', e.target.value)} />
            </div>

            <div className="form-group">
              <label>API URL:</label>
              <input type="url" value={config.api_url} onChange={(e) => updateConfig('api_url', e.target.value)} />
            </div>

            <div className="form-group">
              <label>Global Hotkey:</label>
              <input type="text" value={config.hotkey} onChange={(e) => updateConfig('hotkey', e.target.value)} />
            </div>

            <div className="form-group">
              <label>Microphone:</label>
              <div className="select-wrapper">
                <select value={config.audio_device || ''} onChange={(e) => updateConfig('audio_device', e.target.value || null)}>
                  {availableMics.map(mic => <option key={mic.id} value={mic.id}>{mic.label}</option>)}
                </select>
                <button className="button small icon-button" onClick={loadMics} title="Refresh Devices">
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" strokeWidth="2" stroke="currentColor" fill="none" strokeLinecap="round" strokeLinejoin="round">
                    <path stroke="none" d="M0 0h24v24H0z" fill="none"/>
                    <path d="M20 11a8.1 8.1 0 0 0 -15.5 -2m-.5 -4v4h4" />
                    <path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4" />
                  </svg>
                </button>
              </div>
            </div>

            <div className="form-group">
              <label>Typing Speed (ms):</label>
              <input type="number" value={config.typing_speed_interval} onChange={(e) => updateConfig('typing_speed_interval', parseInt(e.target.value))} />
            </div>

            <div className="form-group">
              <label>Popup Position (px from bottom):</label>
              <input type="number" value={config.pixels_from_bottom} onChange={(e) => updateConfig('pixels_from_bottom', parseInt(e.target.value))} />
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
                    <div className="history-text">{item.text}</div>
                    <div className="history-timestamp">{formatTimestamp(item.timestamp)}</div>
                    <button className="copy-button" onClick={() => copyToClipboard(item.text)}>📋</button>
                  </div>
                ))
              }
            </div>
          </div>
        )}
      </div>

      <div className="toast-container">
        {toasts.map(toast => (
          <div key={toast.id} className={`toast ${toast.type}`} onClick={() => removeToast(toast.id)}>
            <span>{toast.message}</span>
            <button className="toast-close">✕</button>
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
