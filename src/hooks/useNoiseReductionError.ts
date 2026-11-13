import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'preact/hooks';

interface UseNoiseReductionErrorReturn {
  /// Why the last noise reduction attempt failed; null once one succeeds.
  noiseReductionError: string | null;
}

/// Mirrors the backend's record of the last noise reduction outcome, so the
/// settings page can say why recordings are transcribed without it.
export function useNoiseReductionError(): UseNoiseReductionErrorReturn {
  const noiseReductionError = useSignal<string | null>(null);

  useEffect(() => {
    const refresh = () => {
      invoke<string | null>('get_noise_reduction_error')
        .then((error) => { noiseReductionError.value = error; })
        .catch((error) => console.log('Noise reduction status unavailable:', error));
    };
    refresh();
    const unlisten = listen('noise-reduction-status-changed', refresh);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { noiseReductionError: noiseReductionError.value };
}
