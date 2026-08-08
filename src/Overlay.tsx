import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import StatusIcon from './StatusIcon.tsx';
import { tokens } from './design-tokens.ts';

interface StatusUpdatePayload {
  seq: number;
  status: string;
}

type HotkeyMode = 'HoldToTalk' | 'Toggle';

function Overlay() {
  const status = useSignal<string>('Ready');
  const hotkeyMode = useSignal<HotkeyMode>('HoldToTalk');
  let lastStatusSeq = 0;
  const hasTauriRuntime = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in (window as Window & { __TAURI_INTERNALS__?: unknown });
  const isPreviewMode = !hasTauriRuntime;

  const statusLabel = (s: string) => {
    switch (s) {
      case 'Error':
        return 'Mic not found';
      default:
        return s;
    }
  };

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
          const payload = event.payload;
          const nextSeq = typeof payload === 'string' ? lastStatusSeq + 1 : payload.seq;
          const newStatus = typeof payload === 'string' ? payload : payload.status;

          if (newStatus !== 'Recording' && newStatus !== 'Transcribing' && newStatus !== 'Error') {
            return;
          }

          if (nextSeq < lastStatusSeq) {
            return;
          }

          lastStatusSeq = nextSeq;
          status.value = newStatus;
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
          <button type="button" onClick={() => { status.value = 'Recording'; }} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: status.value === 'Recording' ? '#5865f2' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Recording</button>
          <button type="button" onClick={() => { status.value = 'Transcribing'; }} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: status.value === 'Transcribing' ? '#5865f2' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Transcribing</button>
          <button type="button" onClick={() => { status.value = 'Error'; }} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: status.value === 'Error' ? '#ef4444' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Error</button>
        </div>

        <div style={{ width: '260px', height: '140px', border: '1px solid rgba(255,255,255,0.2)', borderRadius: '10px', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', paddingBottom: '0', background: 'rgba(255,255,255,0.04)' }}>
          <div
            key={`overlay-preview-${status.value}`}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '10px',
              isolation: 'isolate',
              contain: 'paint',
              overflow: 'hidden',
              background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
              border: '1px solid rgba(255, 255, 255, 0.1)',
              borderRadius: '999px',
              padding: '6px 12px 6px 8px',
              minWidth: '194px',
            }}
          >
            <StatusIcon status={status.value} size={40} />
            <span style={{ color: '#fff', fontFamily: tokens.typography.fontMain, fontSize: '18px', fontWeight: 500, textAlign: 'center', lineHeight: 1.2, whiteSpace: 'nowrap', textShadow: 'none', flex: 1 }}>
              {statusLabel(status.value)}
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', height: '100vh', width: '100vw', backgroundColor: 'transparent', paddingBottom: '0' }}>
      <div
        key={`overlay-content-${status.value}`}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '10px',
          isolation: 'isolate',
          contain: 'paint',
          overflow: 'hidden',
          background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
          border: '1px solid rgba(255, 255, 255, 0.1)',
          borderRadius: '999px',
          padding: '6px 12px 6px 8px',
          minWidth: '194px',
        }}
      >
        <StatusIcon status={status.value} size={40} />
        <span key={`overlay-status-${status.value}`} style={{ color: '#fff', fontFamily: tokens.typography.fontMain, fontSize: '18px', fontWeight: 500, textAlign: 'center', lineHeight: 1.2, whiteSpace: 'nowrap', textShadow: 'none', flex: 1 }}>
          {statusLabel(status.value)}
          {status.value === 'Recording' && hotkeyMode.value === 'Toggle' && (
            <span style={{ display: 'block', fontSize: tokens.typography.sizeXs, opacity: 0.75 }}>press again to stop</span>
          )}
        </span>
      </div>
    </div>
  );
}

export default Overlay;