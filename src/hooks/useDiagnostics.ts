import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import type { SystemInfo } from '../types.ts';

type ShowToast = (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;

interface UseDiagnosticsReturn {
  systemInfo: SystemInfo | null;
  copyLog: () => Promise<void>;
  openLog: () => Promise<void>;
  openLogFolder: () => Promise<void>;
  copySystemInfo: () => Promise<void>;
}

/// Owns the Settings -> Debug Logs block (App Standard section 4): the session
/// log actions and the system info a bug report asks for.
export function useDiagnostics(showToast?: ShowToast): UseDiagnosticsReturn {
  const systemInfo = useSignal<SystemInfo | null>(null);

  useEffect(() => {
    invoke<SystemInfo>('get_system_info')
      .then((info) => { systemInfo.value = info; })
      .catch((err) => console.error('Failed to read system info:', err));
  }, []);

  const runAction = async (command: string, successMessage: string | null, failureMessage: string) => {
    try {
      await invoke(command);
      if (successMessage) {
        showToast?.(successMessage, 'success');
      }
    } catch (err) {
      console.error(`${failureMessage}:`, err);
      showToast?.(`${failureMessage}: ${err}`, 'error');
    }
  };

  const copyLog = () => runAction('copy_session_log', 'Session log copied to clipboard', 'Failed to copy the session log');
  const openLog = () => runAction('open_session_log', null, 'Failed to open the session log');
  const openLogFolder = () => runAction('open_log_folder', null, 'Failed to open the log folder');
  const copySystemInfo = () => runAction('copy_system_info', 'System info copied to clipboard', 'Failed to copy system info');

  return {
    systemInfo: systemInfo.value,
    copyLog,
    openLog,
    openLogFolder,
    copySystemInfo,
  };
}
