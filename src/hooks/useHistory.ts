import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryItem } from '../types.ts';

interface UseHistoryReturn {
  history: HistoryItem[];
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;
}

export function useHistory(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseHistoryReturn {
  const history = useSignal<HistoryItem[]>([]);

  const loadHistory = async () => {
    try {
      const savedHistory = await invoke<{ items: HistoryItem[] }>('get_history');
      history.value = savedHistory.items || [];
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  };

  const clearHistory = async () => {
    try {
      await invoke('clear_history');
      history.value = [];
      showToast('History cleared', 'success');
    } catch {
      showToast('Failed to clear history', 'error');
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await invoke('plugin:clipboard-manager|write_text', { text });
      showToast('Copied to clipboard', 'success');
    } catch {
      showToast('Failed to copy', 'error');
    }
  };

  return { history: history.value, loadHistory, clearHistory, copyToClipboard };
}