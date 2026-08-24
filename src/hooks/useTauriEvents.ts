import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { listen } from '@tauri-apps/api/event';
import { HotkeyBindingState, ModelDownloadProgress, StatusUpdatePayload, MicVolumePayload } from '../types.ts';

interface UseTauriEventsOptions {
  onSetupStatus: (payload: string) => void;
  onStatusUpdate: (payload: StatusUpdatePayload) => void;
  onHistoryUpdated: () => void;
  onConfigUpdated: () => void;
  onHotkeyBindingState: (payload: HotkeyBindingState) => void;
  onMicTestStarted: () => void;
  onMicTestFinished: () => void;
  onMicVolume: (payload: MicVolumePayload | number) => void;
  onDownloadProgress: (progress: ModelDownloadProgress) => void;
  onPostProcessGpuStatusChanged: () => void;
  onFocus: () => void;
  onHashChange: () => void;
}

export function useTauriEvents(options: UseTauriEventsOptions) {
  const latest = useSignal(options);
  latest.value = options;

  useEffect(() => {
    const unlistenSetup = listen<string>('setup-status', (event) => {
      latest.value.onSetupStatus(event.payload);
    });
    const unlistenStatus = listen<StatusUpdatePayload>('status-update', (event) => {
      latest.value.onStatusUpdate(event.payload);
    });
    const unlistenHistory = listen('history-updated', () => {
      latest.value.onHistoryUpdated();
    });
    const unlistenConfigUpdated = listen('config-updated', () => {
      latest.value.onConfigUpdated();
    });
    const unlistenHotkeyBindingState = listen<HotkeyBindingState>('hotkey-binding-state', (event) => {
      latest.value.onHotkeyBindingState(event.payload);
    });
    const unlistenMicTestStarted = listen('mic-test-playback-started', () => {
      latest.value.onMicTestStarted();
    });
    const unlistenMicTestFinished = listen('mic-test-playback-finished', () => {
      latest.value.onMicTestFinished();
    });
    const unlistenMicVolume = listen<MicVolumePayload | number>('mic-test-volume', (event) => {
      latest.value.onMicVolume(event.payload);
    });
    const unlistenDownloadProgress = listen<ModelDownloadProgress>('model-download-progress', (event) => {
      latest.value.onDownloadProgress(event.payload);
    });
    const unlistenPostProcessGpuStatus = listen('post-process-gpu-status-changed', () => {
      latest.value.onPostProcessGpuStatusChanged();
    });

    const onFocus = () => latest.value.onFocus();
    const onHashChange = () => latest.value.onHashChange();
    window.addEventListener('focus', onFocus);
    window.addEventListener('hashchange', onHashChange);

    return () => {
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('hashchange', onHashChange);
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