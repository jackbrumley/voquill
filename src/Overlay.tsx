
import { useState, useEffect } from 'preact/hooks';
import { listen } from '@tauri-apps/api/event';
import StatusIcon from './StatusIcon.tsx';
import { tokens } from './design-tokens.ts';

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
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', width: '100vw', backgroundColor: 'rgba(0,0,0,0)' }}>
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '4px' }}>
        <StatusIcon status={status} />
        <span
          style={{
            color: '#fff',
            fontFamily: tokens.typography.fontMain,
            fontSize: '16px',
            fontWeight: 300,
            textAlign: 'center',
            background: 'rgba(54, 57, 63, 0.55)',
            padding: '2px 6px',
            borderRadius: '8px',
          }}
        >
          {status}
        </span>
      </div>
    </div>
  );
}

export default Overlay;
