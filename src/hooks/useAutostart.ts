import { useSignal } from '@preact/signals';
import { enable as enableAutostart, disable as disableAutostart, isEnabled } from '@tauri-apps/plugin-autostart';

interface UseAutostartReturn {
  autostartEnabled: boolean;
  toggleAutostart: (enabled: boolean) => Promise<void>;
  loadAutostart: () => Promise<void>;
}

export function useAutostart(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseAutostartReturn {
  const autostartEnabled = useSignal(false);

  const loadAutostart = async () => {
    try {
      autostartEnabled.value = await isEnabled();
    } catch (error: unknown) {
      console.log('Autostart state unavailable:', error);
    }
  };

  const toggleAutostart = async (enabled: boolean) => {
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      autostartEnabled.value = enabled;
      showToast(`Auto-start ${enabled ? 'enabled' : 'disabled'}`, 'success');
    } catch (error) {
      showToast(`Failed to toggle auto-start: ${error}`, 'error');
    }
  };

  return { autostartEnabled: autostartEnabled.value, toggleAutostart, loadAutostart };
}