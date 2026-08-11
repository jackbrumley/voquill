import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { GpuStatus } from '../types.ts';

interface UseGpuStatusReturn {
  gpuStatus: GpuStatus | null;
  postProcessGpuStatus: GpuStatus | null;
  isTestingEngine: boolean;
  refreshGpuStatus: () => Promise<void>;
  refreshPostProcessGpuStatus: () => Promise<void>;
  testTranscriptionEngine: () => Promise<void>;
}

/// Owns GPU availability reporting for both factories. Status is only
/// meaningful after a GPU load attempt, so `testTranscriptionEngine` awaits a
/// real preload (serialized against other warm-ups in the backend) and then
/// refetches — the deterministic "does GPU acceleration work?" probe used by
/// initial setup.
export function useGpuStatus(): UseGpuStatusReturn {
  const gpuStatus = useSignal<GpuStatus | null>(null);
  const postProcessGpuStatus = useSignal<GpuStatus | null>(null);
  const isTestingEngine = useSignal(false);

  const refreshGpuStatus = async () => {
    try {
      gpuStatus.value = await invoke<GpuStatus>('get_gpu_status');
    } catch {
      // Best-effort diagnostic; a missing status is not actionable.
    }
  };

  const refreshPostProcessGpuStatus = async () => {
    try {
      postProcessGpuStatus.value = await invoke<GpuStatus>('get_post_process_gpu_status');
    } catch {
      // Best-effort diagnostic; a missing status is not actionable.
    }
  };

  const testTranscriptionEngine = async () => {
    if (isTestingEngine.value) {
      return;
    }
    isTestingEngine.value = true;
    try {
      await invoke('preload_transcription_engine');
      await refreshGpuStatus();
    } catch {
      // Preload failures are logged backend-side; status stays as reported.
    } finally {
      isTestingEngine.value = false;
    }
  };

  return {
    gpuStatus: gpuStatus.value,
    postProcessGpuStatus: postProcessGpuStatus.value,
    isTestingEngine: isTestingEngine.value,
    refreshGpuStatus,
    refreshPostProcessGpuStatus,
    testTranscriptionEngine,
  };
}
