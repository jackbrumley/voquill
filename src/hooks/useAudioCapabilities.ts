import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { useEffect } from 'preact/hooks';
import type { AudioCapabilities } from '../types.ts';

interface UseAudioCapabilitiesReturn {
  /// Null until the backend has reported what this platform supports.
  audioCapabilities: AudioCapabilities | null;
}

export function useAudioCapabilities(): UseAudioCapabilitiesReturn {
  const audioCapabilities = useSignal<AudioCapabilities | null>(null);

  useEffect(() => {
    invoke<AudioCapabilities>('get_audio_capabilities')
      .then((capabilities) => { audioCapabilities.value = capabilities; })
      .catch((error) => console.log('Audio capabilities unavailable:', error));
  }, []);

  return { audioCapabilities: audioCapabilities.value };
}
