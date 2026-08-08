import { useState, useCallback } from 'preact/hooks';
import { disable as disableAutostart, enable as enableAutostart, isEnabled } from '@tauri-apps/plugin-autostart';

interface UseAutostartReturn {
  autostartEnabled: boolean;
  toggleAutostart: (enabled: boolean) => Promise<void>;
  loadAutostart: () => Promise<void>;
}

export function useAutostart(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseAutostartReturn {
  const [autostartEnabled, setAutostartEnabled] = useState(false);

  const loadAutostart = useCallback(async () => {
    try {
      const enabled = await isEnabled();
      setAutostartEnabled(enabled);
    } catch (error: unknown) {
      console.log('Autostart state unavailable:', error);
    }
  }, []);

  const toggleAutostart = useCallback(async (enabled: boolean) => {
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      setAutostartEnabled(enabled);
      showToast(`Auto-start ${enabled ? 'enabled' : 'disabled'}`, 'success');
    } catch (error) {
      showToast(`Failed to toggle auto-start: ${error}`, 'error');
    }
  }, [showToast]);

  return { autostartEnabled, toggleAutostart, loadAutostart };
}