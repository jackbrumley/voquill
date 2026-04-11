
import { useState, useEffect, useRef } from 'preact/hooks';
import { listen } from '@tauri-apps/api/event';
import StatusIcon from './StatusIcon.tsx';
import { tokens } from './design-tokens.ts';

interface StatusUpdatePayload {
  seq: number;
  status: string;
}

function Overlay() {
  const [status, setStatus] = useState<string>('Ready');
  const lastStatusSeqRef = useRef<number>(0);
  const hasTauriRuntime = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in (window as Window & { __TAURI_INTERNALS__?: unknown });
  const isPreviewMode = !hasTauriRuntime;

  useEffect(() => {
    if (isPreviewMode) {
      return;
    }

    let unlistenStatus: null | (() => void) = null;

    const setupEventListeners = async () => {
      try {
        unlistenStatus = await listen<string | StatusUpdatePayload>('status-update', (event) => {
          const payload = event.payload;
          const nextSeq = typeof payload === 'string' ? lastStatusSeqRef.current + 1 : payload.seq;
          const newStatus = typeof payload === 'string' ? payload : payload.status;

          if (newStatus !== 'Recording' && newStatus !== 'Transcribing') {
            return;
          }

          if (nextSeq < lastStatusSeqRef.current) {
            return;
          }

          lastStatusSeqRef.current = nextSeq;
          setStatus(newStatus);
        });
      } catch (error) {
        console.error('❌ Failed to setup overlay event listeners:', error);
      }
    };

    void setupEventListeners();

    return () => {
      if (unlistenStatus) {
        unlistenStatus();
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
          <button type="button" onClick={() => setStatus('Recording')} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: status === 'Recording' ? '#5865f2' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Recording</button>
          <button type="button" onClick={() => setStatus('Transcribing')} style={{ border: 'none', borderRadius: '8px', padding: '8px 12px', background: status === 'Transcribing' ? '#5865f2' : 'rgba(255,255,255,0.12)', color: '#fff', cursor: 'pointer' }}>Transcribing</button>
        </div>

        <div style={{ width: '260px', height: '140px', border: '1px solid rgba(255,255,255,0.2)', borderRadius: '10px', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', paddingBottom: '0', background: 'rgba(255,255,255,0.04)' }}>
          <div
            key={`overlay-preview-${status}`}
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
            <StatusIcon status={status} size={40} />
            <span style={{ color: '#fff', fontFamily: tokens.typography.fontMain, fontSize: '18px', fontWeight: 500, textAlign: 'center', lineHeight: 1.2, whiteSpace: 'nowrap', textShadow: 'none', flex: 1 }}>
              {status}
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'flex-end', height: '100vh', width: '100vw', backgroundColor: 'transparent', paddingBottom: '0' }}>
      <div
        key={`overlay-content-${status}`}
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
        <StatusIcon status={status} size={40} />
        <span key={`overlay-status-${status}`} style={{ color: '#fff', fontFamily: tokens.typography.fontMain, fontSize: '18px', fontWeight: 500, textAlign: 'center', lineHeight: 1.2, whiteSpace: 'nowrap', textShadow: 'none', flex: 1 }}>
          {status}
        </span>
      </div>
    </div>
  );
}

export default Overlay;
