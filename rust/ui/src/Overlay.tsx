import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import './Overlay.css';

function Overlay() {
  const [status, setStatus] = useState<string>('Ready');

  useEffect(() => {
    let listenersSetup = false;

    const setupEventListeners = async () => {
      if (listenersSetup) return;

      try {
        console.log('🔥 Setting up overlay event listeners...');

        // Listen for status updates
        const unlistenStatus = await listen('status-update', (event) => {
          const newStatus = event.payload as string;
          console.log('🔥 Overlay received status-update:', newStatus);
          setStatus(newStatus);
        });

        // Listen for overlay-specific status updates (fallback)
        const unlistenOverlay = await listen('overlay-status-update', (event) => {
          const newStatus = event.payload as string;
          console.log('🔥 Overlay received overlay-status-update:', newStatus);
          setStatus(newStatus);
        });

        listenersSetup = true;
        console.log('✅ Overlay event listeners setup complete');

        // Cleanup function
        return () => {
          unlistenStatus();
          unlistenOverlay();
        };
      } catch (error) {
        console.error('❌ Failed to setup overlay event listeners:', error);
      }
    };

    setupEventListeners();
  }, []);

  // Apply status class to body
  useEffect(() => {
    const statusClass = getStatusClass(status);
    document.body.className = statusClass;
    
    return () => {
      document.body.className = '';
    };
  }, [status]);

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
    <div className="status-container">
      <div className="overlay-content">
        <div className="icon-circle">
          <span className="status-icon">{getStatusIcon(status)}</span>
        </div>
        <span className="status-text">{status}</span>
      </div>
    </div>
  );
}

export default Overlay;
