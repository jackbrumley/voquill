import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './App.css';

interface Config {
  openai_api_key: string;
  hotkey: string;
  typing_speed_interval: number;
  pixels_from_bottom: number;
}

interface Toast {
  id: number;
  message: string;
  type: 'success' | 'error' | 'info';
}

function App() {
  const [config, setConfig] = useState<Config>({
    openai_api_key: '',
    hotkey: 'ctrl+space',
    typing_speed_interval: 0.01,
    pixels_from_bottom: 100,
  });
  
  const [activeTab, setActiveTab] = useState<'status' | 'config' | 'help'>('status');
  const [isRecording, setIsRecording] = useState(false);
  const [isTestingApi, setIsTestingApi] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');

  // Apply status class to body for animations (like overlay)
  useEffect(() => {
    const statusClass = getStatusClass(currentStatus);
    document.body.className = statusClass;
    
    return () => {
      document.body.className = '';
    };
  }, [currentStatus]);

  // Load configuration on startup
  useEffect(() => {
    loadConfig();
    
    // Listen for hotkey events
    const unlistenPressed = listen('hotkey-pressed', async () => {
      console.log('🎤 Hotkey pressed - starting recording');
      setIsRecording(true);
      setCurrentStatus('Recording');
      try {
        await invoke('start_recording');
      } catch (error) {
        console.error('Failed to start recording:', error);
        showToast(`Failed to start recording: ${error}`, 'error');
        setIsRecording(false);
        setCurrentStatus('Ready');
      }
    });
    
    const unlistenReleased = listen('hotkey-released', async () => {
      console.log('⏹️ Hotkey released - stopping recording');
      try {
        await invoke('stop_recording');
      } catch (error) {
        console.error('Failed to stop recording:', error);
        showToast(`Failed to stop recording: ${error}`, 'error');
      }
      setIsRecording(false);
    });

    // Listen for status updates from backend
    const unlistenStatus = listen('status-update', (event) => {
      const status = event.payload as string;
      console.log('📊 Status update:', status);
      setCurrentStatus(status);
      
      // Reset to ready when typing is complete
      if (status === 'Ready') {
        setIsRecording(false);
      }
    });

    return () => {
      unlistenPressed.then(fn => fn());
      unlistenReleased.then(fn => fn());
      unlistenStatus.then(fn => fn());
    };
  }, []);

  const showToast = (message: string, type: 'success' | 'error' | 'info') => {
    const id = Date.now();
    setToasts(prev => [...prev, { id, message, type }]);
    
    // Auto-remove toast after 3 seconds
    setTimeout(() => {
      setToasts(prev => prev.filter(toast => toast.id !== id));
    }, 3000);
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

  const saveConfig = async () => {
    try {
      const configToSave = {
        openai_api_key: config.openai_api_key || 'your_api_key_here',
        hotkey: config.hotkey,
        typing_speed_interval: config.typing_speed_interval / 1000, // Convert to seconds
        pixels_from_bottom: config.pixels_from_bottom,
      };
      
      await invoke('save_config', { newConfig: configToSave });
      showToast('Configuration saved and hotkey updated!', 'success');
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
      const isValid = await invoke<boolean>('test_api_key', { apiKey: config.openai_api_key });
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

  const toggleRecording = async () => {
    if (isRecording) {
      try {
        await invoke('stop_recording');
      } catch (error) {
        showToast(`Failed to stop recording: ${error}`, 'error');
      }
    } else {
      try {
        await invoke('start_recording');
      } catch (error) {
        showToast(`Failed to start recording: ${error}`, 'error');
      }
    }
  };

  const updateConfig = (field: keyof Config, value: string | number) => {
    setConfig(prev => ({ ...prev, [field]: value }));
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Ready':
        return '✅';
      case 'Recording':
        return '🎤';
      case 'Converting audio':
        return '🔄';
      case 'Transcribing':
        return '🧠';
      case 'Typing':
        return '⌨️';
      default:
        return '📊';
    }
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


  return (
    <div className="app">
      {/* Tab Navigation */}
      <div className="tab-nav">
        <button 
          className={`tab ${activeTab === 'status' ? 'active' : ''}`}
          onClick={() => setActiveTab('status')}
        >
          Status
        </button>
        <button 
          className={`tab ${activeTab === 'config' ? 'active' : ''}`}
          onClick={() => setActiveTab('config')}
        >
          Config
        </button>
        <button 
          className={`tab ${activeTab === 'help' ? 'active' : ''}`}
          onClick={() => setActiveTab('help')}
        >
          Help
        </button>
      </div>

      {/* Tab Content */}
      <div className="tab-content">
        {activeTab === 'status' && (
          <div className="tab-panel">
            <div className="status-display">
              <div className="status-content">
                <span className="status-icon">{getStatusIcon(currentStatus)}</span>
                <span className="status-text">{currentStatus}</span>
              </div>
            </div>
            
            <div className="record-section">
              <div className="help-text">
                Hotkey: <strong>{config.hotkey}</strong>
              </div>
            </div>

          </div>
        )}

        {activeTab === 'config' && (
          <div className="tab-panel">
            <div className="form-group">
              <label htmlFor="api-key">OpenAI API Key:</label>
              <input
                type="password"
                id="api-key"
                placeholder="sk-..."
                value={config.openai_api_key}
                onChange={(e) => updateConfig('openai_api_key', e.target.value)}
              />
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

            <div className="form-actions">
              <button className="button primary" onClick={saveConfig}>
                💾 Save Configuration
              </button>
              <button 
                className="button" 
                onClick={testApiKey}
                disabled={isTestingApi}
              >
                {isTestingApi ? '🔄 Testing...' : '🧪 Test API Key'}
              </button>
            </div>
          </div>
        )}

        {activeTab === 'help' && (
          <div className="tab-panel">
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
                  <li>Right-click tray icon to reopen</li>
                  <li>Adjust typing speed for better compatibility</li>
                  <li>Test your API key before first use</li>
                </ul>
              </div>
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
