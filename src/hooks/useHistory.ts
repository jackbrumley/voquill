import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryItem } from '../types.ts';

interface UseHistoryReturn {
  history: HistoryItem[];
  searchQuery: string;
  searchResults: HistoryItem[];
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  copyToClipboard: (text: string) => Promise<void>;
  setSearchQuery: (query: string) => void;
}

export function useHistory(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseHistoryReturn {
  const history = useSignal<HistoryItem[]>([]);
  const searchQuery = useSignal<string>('');
  const searchResults = useSignal<HistoryItem[]>([]);

  const loadHistory = async () => {
    try {
      const items = await invoke<HistoryItem[]>('get_history');
      history.value = items || [];
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  };

  useSignalEffect(() => {
    const query = searchQuery.value;
    const allItems = history.value;
    if (!query.trim()) {
      searchResults.value = allItems;
      return;
    }
    const timer = setTimeout(async () => {
      try {
        const items = await invoke<HistoryItem[]>('search_history', { query });
        if (searchQuery.peek() === query) {
          searchResults.value = items || [];
        }
      } catch (error) {
        console.error('Failed to search history:', error);
      }
    }, 300);
    return () => clearTimeout(timer);
  });

  const setSearchQuery = (query: string) => {
    searchQuery.value = query;
  };

  const clearHistory = async () => {
    try {
      await invoke('clear_history');
      history.value = [];
      searchResults.value = [];
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

  return {
    history: history.value,
    searchQuery: searchQuery.value,
    searchResults: searchResults.value,
    loadHistory,
    clearHistory,
    copyToClipboard,
    setSearchQuery,
  };
}