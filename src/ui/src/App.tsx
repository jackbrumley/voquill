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

function App() {
  const [config, setConfig] = useState<Config>({
    openai_api_key: '',
    api_url: 'https://api.openai.com/v1/audio/transcriptions',
    hotkey: 'ctrl+space',
    typing_speed_interval: 0.01,
    pixels_from_bottom: 100,
  });
  
  const [activeTab, setActiveTab] = useState<'status' | 'history' | 'config'>('status');
  const [isTestingApi, setIsTestingApi] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [history, setHistory] = useState<HistoryItem[]>([]);

  // Apply status class to body for animations (like overlay)
  useEffect(() => {
    const statusClass = getStatusClass(currentStatus);
    document.body.className = statusClass;
    
    return () => {
      document.body.className = '';
    };
  }, [currentStatus]);

  // Load configuration and history on startup
  useEffect(() => {
    loadConfig();
    loadHistory();
    
    // Listen for hotkey events
    const unlistenPressed = listen('hotkey-pressed', async () => {
      console.log('🎤 Hotkey pressed - starting recording');
      setCurrentStatus('Recording');
      try {
        await invoke('start_recording');
      } catch (error) {
        console.error('Failed to start recording:', error);
        setCurrentStatus('Ready');
      }
    });
    
    const unlistenReleased = listen('hotkey-released', async () => {
      console.log('⏹️ Hotkey released - stopping recording');
      try {
        await invoke('stop_recording');
      } catch (error) {
        console.error('Failed to stop recording:', error);
      }
    });

    // Listen for system setup status
    const unlistenSetup = listen('setup-status', (event) => {
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
    const unlistenHotkeyError = listen('hotkey-error', (event) => {
      const error = event.payload as string;
      if (error.startsWith('conflict')) {
        const key = error.split(':')[1] || 'shortcut';
        showToast(`⚠️ Hotkey Conflict: '${key}' is used by another app. Please change it in Config.`, 'error');
      } else {
        showToast(`❌ Hotkey Error: ${error.split(':')[1] || 'Failed to register'}`, 'error');
      }
    });

    // Listen for audio errors
    const unlistenAudioError = listen('audio-error', (event) => {
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
    const unlistenStatus = listen('status-update', (event) => {
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
      unlistenPressed.then(fn => fn());
      unlistenReleased.then(fn => fn());
      unlistenSetup.then(fn => fn());
      unlistenHotkeyError.then(fn => fn());
      unlistenAudioError.then(fn => fn());
      unlistenStatus.then(fn => fn());
      unlistenHistory.then(fn => fn());
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
    setToasts(prev => [...prev, { id, message, type }]);
    
    // Auto-remove toast after 5 seconds
    setTimeout(() => {
      setToasts(prev => prev.filter(toast => toast.id !== id));
    }, 5000);
  };

  const removeToast = (id: number) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
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
      console.log('📚 Loaded history:', historyData.items.length, 'items');
    } catch (error) {
      console.error('Failed to load history:', error);
      showToast('Failed to load history', 'error');
    }
  };

  const clearHistory = async () => {
    try {
      await invoke('clear_history');
      setHistory([]);
      showToast('History cleared successfully', 'success');
    } catch (error) {
      console.error('Failed to clear history:', error);
      showToast('Failed to clear history', 'error');
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast('Copied to clipboard!', 'success');
    } catch (error) {
      console.error('Failed to copy to clipboard:', error);
      showToast('Failed to copy to clipboard', 'error');
    }
  };

  const saveConfig = async () => {
    try {
      const configToSave = {
        openai_api_key: config.openai_api_key || 'your_api_key_here',
        api_url: config.api_url,
        hotkey: config.hotkey,
        typing_speed_interval: config.typing_speed_interval / 1000, // Convert to seconds
        pixels_from_bottom: config.pixels_from_bottom,
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

    if (!config.api_url) {
      showToast('Please enter an API URL first', 'error');
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


  const updateConfig = (field: keyof Config, value: string | number) => {
    setConfig(prev => ({ ...prev, [field]: value }));
  };

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'Ready':
        return 'status-ready';
      case 'Recording':
        return 'status-recording';
      case 'Converting audio':
      case 'Transcribing':
        return 'status-transcribing';
      case 'Typing':
        return 'status-typing';
      default:
        return '';
    }
  };

  const formatTimestamp = (timestamp: string) => {
    try {
      // Parse the ISO 8601 UTC timestamp and convert to local time
      const date = new Date(timestamp);
      return date.toLocaleString();
    } catch (error) {
      // Fallback for any parsing errors
      return timestamp;
    }
  };

  const handleMinimize = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.minimize();
    } catch (error) {
      console.error('Failed to minimize window:', error);
    }
  };

  const handleClose = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.close();
    } catch (error) {
      console.error('Failed to close window:', error);
    }
  };

  const handleTitleBarMouseDown = async (e: React.MouseEvent) => {
    // Only start dragging if it's a left click and not on a button
    if (e.button === 0 && !(e.target as HTMLElement).closest('.title-bar-button')) {
      try {
        const appWindow = getCurrentWindow();
        await appWindow.startDragging();
      } catch (error) {
        console.error('Failed to start dragging:', error);
      }
    }
  };

  return (
    <div className="app">
      {/* Custom Title Bar */}
      <div className="title-bar" onMouseDown={handleTitleBarMouseDown}>
        <div className="title-bar-title">Voquill</div>
        <div className="title-bar-controls">
          <button className="title-bar-button minimize" onClick={handleMinimize} title="Minimize">
            ─
          </button>
          <button className="title-bar-button maximize" onClick={() => {}} title="Maximize">
            ☐
          </button>
          <button className="title-bar-button close" onClick={handleClose} title="Close">
            ✕
          </button>
        </div>
      </div>
      {/* Tab Navigation */}
      <div className="tab-nav">
        <button 
          className={`tab ${activeTab === 'status' ? 'active' : ''}`}
          onClick={() => setActiveTab('status')}
        >
          Status
        </button>
        <button 
          className={`tab ${activeTab === 'history' ? 'active' : ''}`}
          onClick={() => setActiveTab('history')}
        >
          History
        </button>
        <button 
          className={`tab ${activeTab === 'config' ? 'active' : ''}`}
          onClick={() => setActiveTab('config')}
        >
          Config
        </button>
      </div>

      {/* Tab Content */}
      <div className="tab-content">
        {activeTab === 'status' && (
          <div className="tab-panel">
            <div className="status-display">
              <StatusIcon status={currentStatus} className="app-status-icon" />
              <div className="status-text-app">{currentStatus}</div>
            </div>
            
            <div className="record-section">
              <div className="help-text">
                Hotkey: <strong>{config.hotkey}</strong>
              </div>
            </div>

            <div className="help-content">
              <h3>How to Use Voquill</h3>
              <ol className="instructions">
                <li>Enter your OpenAI API key in the Config tab</li>
                <li>Save your configuration</li>
                <li>Position cursor in any text field</li>
                <li>Hold <strong>{config.hotkey}</strong> and speak</li>
                <li>Release keys when done speaking</li>
                <li>Text will be typed automatically</li>
              </ol>
              
              <div className="help-section">
                <h4>Tips</h4>
                <ul>
                  <li>App runs in system tray when closed</li>
                  <li>Left-click tray icon to reopen</li>
                  <li>Adjust typing speed for better compatibility</li>
                  <li>Test your API key before first use</li>
                </ul>
              </div>
            </div>

          </div>
        )}

        {activeTab === 'config' && (
          <div className="tab-panel">
            <div className="form-actions">
              <button className="button primary" onClick={saveConfig}>
                Save Configuration
              </button>
              <button 
                className="button" 
                onClick={testApiKey}
                disabled={isTestingApi}
              >
                {isTestingApi ? 'Testing...' : 'Test API Key'}
              </button>
            </div>

            <div className="form-group">
              <label htmlFor="api-key">API Key:</label>
              <input
                type="password"
                id="api-key"
                placeholder="sk-..."
                value={config.openai_api_key}
                onChange={(e) => updateConfig('openai_api_key', e.target.value)}
              />
            </div>

            <div className="form-group">
              <label htmlFor="api-url">API URL:</label>
              <input
                type="url"
                id="api-url"
                placeholder="https://api.openai.com/v1/audio/transcriptions"
                value={config.api_url}
                onChange={(e) => updateConfig('api_url', e.target.value)}
              />
              <small className="form-help">
                Use OpenAI, OpenRouter, or any compatible Whisper API endpoint
              </small>
            </div>

            <div className="form-group">
              <label htmlFor="hotkey">Global Hotkey:</label>
              <input
                type="text"
                id="hotkey"
                placeholder="ctrl+space"
                value={config.hotkey}
                onChange={(e) => updateConfig('hotkey', e.target.value)}
              />
            </div>

            <div className="form-group">
              <label htmlFor="typing-speed">Typing Speed (ms):</label>
              <input
                type="number"
                id="typing-speed"
                min="1"
                max="1000"
                step="1"
                value={config.typing_speed_interval}
                onChange={(e) => updateConfig('typing_speed_interval', parseInt(e.target.value))}
              />
            </div>

            <div className="form-group">
              <label htmlFor="popup-position">Popup Position (px from bottom):</label>
              <input
                type="number"
                id="popup-position"
                min="50"
                max="500"
                step="10"
                value={config.pixels_from_bottom}
                onChange={(e) => updateConfig('pixels_from_bottom', parseInt(e.target.value))}
              />
            </div>
          </div>
        )}

        {activeTab === 'history' && (
          <div className="tab-panel">
            <div className="history-header">
              <button 
                className="button clear-history-button" 
                onClick={clearHistory}
              >
                Clear History
              </button>
            </div>
            
            <div className="history-list">
              {history.length === 0 ? (
                <div className="empty-history">
                  <p>No transcriptions yet. Start recording to see your history here!</p>
                </div>
              ) : (
                history.map((item) => (
                  <div key={item.id} className="history-item">
                    <div className="history-content">
                      <div className="history-text">{item.text}</div>
                      <div className="history-timestamp">{formatTimestamp(item.timestamp)}</div>
                    </div>
                    <button 
                      className="copy-button"
                      onClick={() => copyToClipboard(item.text)}
                      title="Copy to clipboard"
                    >
                      📋
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      {/* Toast Notifications */}
      <div className="toast-container">
        {toasts.map(toast => (
          <div 
            key={toast.id} 
            className={`toast ${toast.type}`}
            onClick={() => removeToast(toast.id)}
          >
            <span>{toast.message}</span>
            <button className="toast-close">✕</button>
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
