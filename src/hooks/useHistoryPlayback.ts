import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { HistoryItem } from '../types.ts';

type ShowToast = (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;

interface UseHistoryPlaybackReturn {
  /// History item whose recording is playing, if any.
  playingItemId: number | null;
  togglePlayback: (item: HistoryItem) => Promise<void>;
}

/// History recordings play through the backend on the configured playback
/// device. The backend emits `history-playback-finished` with the file name
/// whenever a recording ends (played out, stopped, or replaced by another
/// preview), which is the only thing that clears the playing state.
export function useHistoryPlayback(showToast?: ShowToast): UseHistoryPlaybackReturn {
  const playingItemId = useSignal<number | null>(null);
  const playingFileName = useSignal<string | null>(null);

  useEffect(() => {
    const unlistenFinished = listen<string>('history-playback-finished', (event) => {
      if (event.payload === playingFileName.value) {
        playingItemId.value = null;
        playingFileName.value = null;
      }
    });
    return () => {
      unlistenFinished.then((unlisten) => unlisten());
      if (playingFileName.value) {
        void invoke('stop_history_recording');
      }
    };
  }, []);

  const togglePlayback = async (item: HistoryItem) => {
    if (!item.audio_file) {
      return;
    }
    if (playingItemId.value === item.id) {
      await invoke('stop_history_recording');
      return;
    }

    // Set before invoking: starting this recording ends the previous one,
    // whose finished event must not be mistaken for this one's.
    playingItemId.value = item.id;
    playingFileName.value = item.audio_file;
    try {
      await invoke('play_history_recording', { fileName: item.audio_file });
    } catch (error) {
      if (playingItemId.value === item.id) {
        playingItemId.value = null;
        playingFileName.value = null;
      }
      showToast?.(`Failed to play recording: ${error}`, 'error');
    }
  };

  return {
    playingItemId: playingItemId.value,
    togglePlayback,
  };
}
