import { useState, useCallback } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryItem } from '../types.ts';

interface UseHistoryReturn {
  history: HistoryItem[];
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;
}

export function useHistory(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseHistoryReturn {
  const [history, setHistory] = useState<HistoryItem[]>([]);

  const loadHistory = useCallback(async () => {
    try {
      const savedHistory = await invoke<{ items: HistoryItem[] }>('get_history');
      setHistory(savedHistory.items || []);
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  }, []);

  const clearHistory = useCallback(async () => {
    try {
      await invoke('clear_history');
      setHistory([]);
      showToast('History cleared', 'success');
    } catch {
      showToast('Failed to clear history', 'error');
    }
  }, [showToast]);

  const copyToClipboard = useCallback(async (text: string) => {
    try {
      await invoke('plugin:clipboard-manager|write_text', { text });
      showToast('Copied to clipboard', 'success');
    } catch {
      showToast('Failed to copy', 'error');
    }
  }, [showToast]);

  return { history, loadHistory, clearHistory, copyToClipboard };
}