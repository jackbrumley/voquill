import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StatusIndicator } from './components/StatusIndicator.tsx';
import { tokens } from './design-tokens.ts';
import { useDictationStatus } from './hooks/useDictationStatus.ts';
import type { StatusUpdatePayload } from './types.ts';

type HotkeyMode = 'HoldToTalk' | 'Toggle';

function Overlay() {
  const dictationStatus = useDictationStatus({ accept: ['Recording', 'Transcribing', 'Error'] });
  const hotkeyMode = useSignal<HotkeyMode>('HoldToTalk');
  const hasTauriRuntime = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in (window as Window & { __TAURI_INTERNALS__?: unknown });
  const isPreviewMode = !hasTauriRuntime;

  useEffect(() => {
    if (isPreviewMode) {
      return;
    }

    let unlistenStatus: null | (() => void) = null;
    let unlistenConfig: null | (() => void) = null;

    const loadHotkeyMode = async () => {
      try {
        const config = await invoke<{ hotkey_mode: HotkeyMode }>('get_config');
        hotkeyMode.value = config.hotkey_mode;
      } catch (error) {
        console.error('Failed to load overlay config:', error);
      }
    };

    const setupEventListeners = async () => {
      try {
        void loadHotkeyMode();
        unlistenConfig = await listen('config-updated', () => {
          void loadHotkeyMode();
        });

        unlistenStatus = await listen<string | StatusUpdatePayload>('status-update', (event) => {
          dictationStatus.handleStatusUpdate(event.payload);
        });
      } catch (error) {
        console.error('Failed to setup overlay event listeners:', error);
      }
    };

    void setupEventListeners();

    return () => {
      if (unlistenStatus) {
        unlistenStatus();
      }
      if (unlistenConfig) {
        unlistenConfig();
      }
    };
  }, [isPreviewMode]);

  useEffect(() => {
    if (isPreviewMode) {
      return;
    }

    const htmlEl = document.documentElement;
    const bodyEl = document.body;
    const rootEl = document.getElementById('root');

    if (htmlEl) {
      htmlEl.style.background = 'transparent';
    }
    if (bodyEl) {
      bodyEl.style.background = 'transparent';
    }
    if (rootEl) {
      (rootEl as HTMLElement).style.background = 'transparent';
    }
  }, [isPreviewMode]);

  if (isPreviewMode) {
    return (
      <div style={{ minHeight: '100vh', width: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '16px', background: '#1f2125', color: '#fff', fontFamily: tokens.typography.fontMain }}>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button type="button" onClick={() => { dictationStatus.status.value = 'Recording'; }} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: dictationStatus.status.value === 'Recording' ? '#5865f2' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Recording</button>
          <button type="button" onClick={() => { dictationStatus.status.value = 'Transcribing'; }} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: dictationStatus.status.value === 'Transcribing' ? '#5865f2' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Transcribing</button>
          <button type="button" onClick={() => { dictationStatus.status.value = 'Error'; }} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: dictationStatus.status.value === 'Error' ? '#ef4444' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Error</button>
        </div>

        <div style={{ width: '260px', height: '140px', border: '1px solid rgba(255,255,255,0.2)', borderRadius: '10px', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', padding: '12px', background: 'rgba(255,255,255,0.04)' }}>
            <StatusIndicator status={dictationStatus.status.value} size={40} />
          </div>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', width: '100%', height: '100%', backgroundColor: 'transparent', padding: '20px 20px 0' }}>
      <StatusIndicator status={dictationStatus.status.value} size={44} fixedWidth={180} subtitle={dictationStatus.status.value === 'Recording' && hotkeyMode.value === 'Toggle' ? 'press again to stop' : undefined} />
    </div>
  );
}

export default Overlay;
