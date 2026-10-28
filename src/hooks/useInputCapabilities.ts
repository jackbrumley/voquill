import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { useEffect } from 'preact/hooks';
import type { InputCapabilities } from '../types.ts';

interface UseInputCapabilitiesReturn {
  /// Null until the backend has reported what the active platform supports.
  inputCapabilities: InputCapabilities | null;
}

export function useInputCapabilities(): UseInputCapabilitiesReturn {
  const inputCapabilities = useSignal<InputCapabilities | null>(null);

  useEffect(() => {
    invoke<InputCapabilities>('get_input_capabilities')
      .then((capabilities) => { inputCapabilities.value = capabilities; })
      .catch((error) => console.log('Input capabilities unavailable:', error));
  }, []);

  return { inputCapabilities: inputCapabilities.value };
}
