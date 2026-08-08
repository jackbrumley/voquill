import { useState, useCallback, useRef } from 'preact/hooks';
import { Toast } from '../types.ts';
import { getToastMessageStyle, getToastStyle, toastContainerStyle } from '../theme/ui-primitives.ts';

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextToastId = useRef(0);

  const showToast = useCallback((message: string, type: Toast['type']) => {
    const id = nextToastId.current++;
    setToasts((prev) => [...prev, { id, message, type }]);
    const duration = type === 'error' ? 10000 : type === 'saved' ? 900 : 3000;
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, duration);
  }, []);

  const handleToastClick = useCallback((toast: Toast) => {
    if (toast.type === 'saved') {
      setToasts((prev) => prev.filter((t) => t.id !== toast.id));
    }
  }, []);

  const ToastContainer = () => (
    <div style={toastContainerStyle}>
      {toasts.map((toast) => (
        <div
          key={toast.id}
          style={getToastStyle(toast.type)}
          onClick={() => handleToastClick(toast)}
        >
          <span style={getToastMessageStyle(toast.type)}>{toast.message}</span>
        </div>
      ))}
    </div>
  );

  return { toasts, showToast, ToastContainer };
}