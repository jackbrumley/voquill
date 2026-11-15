import { useEffect } from 'preact/hooks';
import { useSignal, type ReadonlySignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { READINESS_NOT_YET_CHECKED, type ReadinessStatus } from '../readiness.ts';

interface UseAppReadinessReturn {
  readiness: ReadinessStatus;
  /** True once the backend's first verdict has arrived (a signal, so the
   *  startup routing computation re-runs when it flips). */
  hasLoadedReadiness: ReadonlySignal<boolean>;
  /** Asks the backend to re-evaluate now (e.g. when the window regains focus). */
  refreshReadiness: () => Promise<void>;
}

/// Mirrors the backend's readiness verdict: loads it once and follows
/// `app-readiness-changed`, which the backend sends whenever a permission,
/// device, model, hotkey or setting changes it.
export function useAppReadiness(): UseAppReadinessReturn {
  const readiness = useSignal<ReadinessStatus>(READINESS_NOT_YET_CHECKED);
  const hasLoadedReadiness = useSignal(false);

  const refreshReadiness = async () => {
    try {
      readiness.value = await invoke<ReadinessStatus>('get_app_readiness');
      hasLoadedReadiness.value = true;
    } catch (error) {
      console.error('Failed to load app readiness:', error);
    }
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<ReadinessStatus>('app-readiness-changed', (event) => {
      readiness.value = event.payload;
      hasLoadedReadiness.value = true;
    }).then((stop) => {
      if (cancelled) stop();
      else unlisten = stop;
    }).catch((error: unknown) => {
      console.error('Failed to listen for readiness changes:', error);
    });
    void refreshReadiness();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return { readiness: readiness.value, hasLoadedReadiness, refreshReadiness };
}
