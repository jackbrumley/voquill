
import { useState, useEffect } from 'preact/hooks';
import { listen } from '@tauri-apps/api/event';
import StatusIcon from './StatusIcon.tsx';
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
        const unlistenStatus = await listen<string>('status-update', (event) => {
          const newStatus = event.payload;
          console.log('🔥 Overlay received status-update:', newStatus);
          setStatus(newStatus);
        });

        // Listen for overlay-specific status updates (fallback)
        const unlistenOverlay = await listen<string>('overlay-status-update', (event) => {
          const newStatus = event.payload;
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

  return (
    <div className="status-container">
      <div className="overlay-content">
        <StatusIcon status={status} />
        <span className="status-text">{status}</span>
      </div>
    </div>
  );
}

export default Overlay;
