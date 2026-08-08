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

    invoke<{ status: string; status_seq?: number }>('get_current_status')
      .then((data) => {
        status.value = data.status;
        if (typeof data.status_seq === 'number') {
          lastStatusSeq = data.status_seq;
        }
      })
      .catch(() => {});

    invoke<{ hotkey_mode: HotkeyMode }>('get_config')
      .then((data) => { hotkeyMode.value = data.hotkey_mode; })
      .catch(() => {});

    const unlisten = listen<StatusUpdatePayload>('status-update', (event) => {
      const payload = event.payload;
      const nextStatus = typeof payload === 'string' ? payload : payload.status;
      const nextSeq = typeof payload === 'object' ? (payload as StatusUpdatePayload).seq : undefined;

      if (typeof nextSeq === 'number' && nextSeq < lastStatusSeq) {
        return;
      }
      if (typeof nextSeq === 'number') {
        lastStatusSeq = nextSeq;
      }
      status.value = nextStatus;
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div style={{ minHeight: '100vh', width: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '16px', background: '#1f2125', color: '#fff', fontFamily: tokens.typography.fontMain }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '12px' }}>
        <StatusIcon status={status.value} />
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start' }}>
          <div style={{ fontWeight: 600, fontSize: '32px', lineHeight: 1.1 }}>
            {statusLabel(status.value)}
          </div>
          <span style={{ color: '#999', fontSize: '13px', lineHeight: 1.2, marginTop: '2px' }}>
            Voquill
          </span>
        </div>
      </div>

      <div style={{
        background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
        borderRadius: '999px',
        padding: '3px',
        display: 'flex',
        gap: '2px',
        marginTop: '8px',
      }}>
        <div style={{
          padding: '5px 12px',
          borderRadius: '999px',
          fontWeight: 500,
          fontSize: '12px',
          background: status.value === 'Recording' || (status.value !== 'Ready' && status.value !== 'Transcribing' && status.value !== 'Error') ? 'rgba(0,0,0,0.3)' : 'transparent',
        }}>
          {hotkeyMode.value === 'Toggle' ? 'Toggle' : 'Hold'}
        </div>
        <div style={{
          padding: '5px 12px',
          borderRadius: '999px',
          fontWeight: 500,
          fontSize: '12px',
          background: status.value === 'Transcribing' ? 'rgba(0,0,0,0.3)' : 'transparent',
        }}>
          {status.value === 'Transcribing' ? 'Working...' : status.value === 'Recording' ? 'Recording' : status.value}
        </div>
      </div>

      {status.value === 'Recording' && (
        <span key={`overlay-status-${status.value}`} style={{ color: '#fff', fontFamily: tokens.typography.fontMain, fontSize: '18px', fontWeight: 500, textAlign: 'center', lineHeight: 1.2, whiteSpace: 'nowrap', textShadow: 'none', flex: 1 }}>
          {hotkeyMode.value === 'HoldToTalk' ? 'Release to stop' : 'Press hotkey again to stop'}
          <span style={{ display: 'block', fontSize: tokens.typography.sizeXs, opacity: 0.75 }}>press again to stop</span>
        </span>
      )}
    </div>
  );
}

export default Overlay;