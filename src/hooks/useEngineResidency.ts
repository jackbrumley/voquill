import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { EngineResidency } from '../types.ts';

export interface UseEngineResidencyReturn {
  residency: EngineResidency | null;
  loadEngine: () => Promise<void>;
  unloadEngine: () => Promise<void>;
}

/// Owns the speech engine's loaded state. The backend decides it and pushes
/// every change with `engine-residency-changed`; this hook only mirrors it and
/// forwards load/unload requests.
export function useEngineResidency(): UseEngineResidencyReturn {
  const residency = useSignal<EngineResidency | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    listen<EngineResidency>('engine-residency-changed', (event) => {
      residency.value = event.payload;
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unlisten = stop;
      }
    }).catch((error: unknown) => {
      console.error('Failed to listen for speech engine changes:', error);
    });

    invoke<EngineResidency>('get_engine_residency')
      .then((current) => {
        residency.value = current;
      })
      .catch((error: unknown) => {
        console.error('Failed to read speech engine state:', error);
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const loadEngine = async () => {
    try {
      await invoke('preload_transcription_engine');
    } catch (error) {
      console.error('Failed to load speech engine:', error);
    }
  };

  const unloadEngine = async () => {
    try {
      await invoke('unload_model');
    } catch (error) {
      console.error('Failed to unload speech engine:', error);
    }
  };

  return { residency: residency.value, loadEngine, unloadEngine };
}

/// Plain-language description of the idle unload setting.
export function describeIdleUnload(minutes: number): string {
  if (minutes <= 0) {
    return 'It stays loaded until you unload it.';
  }
  if (minutes % 60 === 0) {
    const hours = minutes / 60;
    return `It unloads by itself after ${hours} hour${hours === 1 ? '' : 's'} without use.`;
  }
  return `It unloads by itself after ${minutes} minutes without use.`;
}

/// Shown beside the idle unload setting while voice macros are on: the
/// backend pauses them while the engine is unloaded.
export const VOICE_MACRO_PAUSE_NOTE = 'Voice macros pause while the speech engine is unloaded.';
