import { useSignal } from '@preact/signals';
import type { JSX } from 'preact';
import { Toast } from '../types.ts';
import { getToastMessageStyle, getToastStyle, toastContainerStyle } from '../theme/ui-primitives.ts';

const closeButtonStyle: JSX.CSSProperties = {
  background: 'none',
  border: 'none',
  color: 'rgba(255,255,255,0.7)',
  fontSize: '14px',
  lineHeight: 1,
  cursor: 'pointer',
  padding: '2px',
  flexShrink: 0,
  opacity: 0.7,
  transition: 'opacity 0.15s ease',
};

export function useToast() {
  const toasts = useSignal<Toast[]>([]);
  let nextToastId = 0;

  const showToast = (message: string, type: Toast['type']) => {
    const id = nextToastId++;
    toasts.value = [...toasts.value, { id, message, type }];
    const duration = type === 'error' ? 10000 : type === 'saved' ? 900 : 3000;
    setTimeout(() => {
      toasts.value = toasts.value.filter((t) => t.id !== id);
    }, duration);
  };

  const dismissToast = (id: number) => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  };

  const handleToastClick = (toast: Toast) => {
    if (toast.type === 'saved') {
      dismissToast(toast.id);
    }
  };

  const ToastContainer = () => (
    <div style={toastContainerStyle}>
      {toasts.value.map((toast) => (
        <div
          key={toast.id}
          style={{ ...getToastStyle(toast.type), display: 'flex', alignItems: 'center', gap: '10px', paddingRight: '12px' }}
          onClick={() => handleToastClick(toast)}
        >
          <span style={{ flex: 1, ...getToastMessageStyle(toast.type) }}>{toast.message}</span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              dismissToast(toast.id);
            }}
            style={closeButtonStyle}
            aria-label="Dismiss"
          >
            ✕
          </button>
        </div>
      ))}
    </div>
  );

  return { showToast, ToastContainer };
}