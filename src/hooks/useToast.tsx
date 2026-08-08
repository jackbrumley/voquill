import { useSignal } from '@preact/signals';
import { Toast } from '../types.ts';
import { getToastMessageStyle, getToastStyle, toastContainerStyle } from '../theme/ui-primitives.ts';

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

  const handleToastClick = (toast: Toast) => {
    if (toast.type === 'saved') {
      toasts.value = toasts.value.filter((t) => t.id !== toast.id);
    }
  };

  const ToastContainer = () => (
    <div style={toastContainerStyle}>
      {toasts.value.map((toast) => (
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

  return { showToast, ToastContainer };
}