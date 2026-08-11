import { useEffect } from 'preact/hooks';
import { listen } from '@tauri-apps/api/event';
import { HotkeyBindingState, ModelDownloadProgress, StatusUpdatePayload } from '../types.ts';

interface UseTauriEventsOptions {
  onSetupStatus: (payload: string) => void;
  onStatusUpdate: (payload: StatusUpdatePayload) => void;
  onHistoryUpdated: () => void;
  onConfigUpdated: () => void;
  onHotkeyBindingState: (payload: HotkeyBindingState) => void;
  onMicTestStarted: () => void;
  onMicTestFinished: () => void;
  onMicVolume: (volume: number) => void;
  onDownloadProgress: (progress: ModelDownloadProgress) => void;
  onPostProcessGpuStatusChanged: () => void;
  onFocus: () => void;
  onHashChange: () => void;
}

export function useTauriEvents(options: UseTauriEventsOptions) {
  useEffect(() => {
    const unlistenSetup = listen<string>('setup-status', (event) => {
      options.onSetupStatus(event.payload);
    });
    const unlistenStatus = listen<StatusUpdatePayload>('status-update', (event) => {
      options.onStatusUpdate(event.payload);
    });
    const unlistenHistory = listen('history-updated', () => {
      options.onHistoryUpdated();
    });
    const unlistenConfigUpdated = listen('config-updated', () => {
      options.onConfigUpdated();
    });
    const unlistenHotkeyBindingState = listen<HotkeyBindingState>('hotkey-binding-state', (event) => {
      options.onHotkeyBindingState(event.payload);
    });
    const unlistenMicTestStarted = listen('mic-test-playback-started', () => {
      options.onMicTestStarted();
    });
    const unlistenMicTestFinished = listen('mic-test-playback-finished', () => {
      options.onMicTestFinished();
    });
    const unlistenMicVolume = listen<number>('mic-test-volume', (event) => {
      options.onMicVolume(event.payload);
    });
    const unlistenDownloadProgress = listen<ModelDownloadProgress>('model-download-progress', (event) => {
      options.onDownloadProgress(event.payload);
    });
    const unlistenPostProcessGpuStatus = listen('post-process-gpu-status-changed', () => {
      options.onPostProcessGpuStatusChanged();
    });

    window.addEventListener('focus', options.onFocus);
    window.addEventListener('hashchange', options.onHashChange);

    return () => {
      window.removeEventListener('focus', options.onFocus);
      window.removeEventListener('hashchange', options.onHashChange);
      unlistenSetup.then((fn) => fn());
      unlistenStatus.then((fn) => fn());
      unlistenHistory.then((fn) => fn());
      unlistenConfigUpdated.then((fn) => fn());
      unlistenHotkeyBindingState.then((fn) => fn());
      unlistenMicTestStarted.then((fn) => fn());
      unlistenMicTestFinished.then((fn) => fn());
      unlistenMicVolume.then((fn) => fn());
      unlistenDownloadProgress.then((fn) => fn());
      unlistenPostProcessGpuStatus.then((fn) => fn());
    };
  }, []);
}