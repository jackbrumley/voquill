import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import type { UpdateCheckResult } from '../types.ts';

interface UseUpdatesReturn {
  updateResult: UpdateCheckResult | null;
  lastCheckedAt: number | null;
  checkingUpdates: boolean;
  showUpdateModal: boolean;
  setShowUpdateModal: (show: boolean) => void;
  checkForUpdates: (showUpToDateModal: boolean) => Promise<void>;
  openLatestReleasePage: () => Promise<void>;
  getLastCheckedLabel: () => string;
}

export function useUpdates(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseUpdatesReturn {
  const updateResult = useSignal<UpdateCheckResult | null>(null);
  const lastCheckedAt = useSignal<number | null>(null);
  const checkingUpdates = useSignal(false);
  const showUpdateModal = useSignal(false);

  const checkForUpdates = async (showUpToDateModal: boolean) => {
    if (checkingUpdates.value) return;

    checkingUpdates.value = true;
    try {
      const result = await invoke<UpdateCheckResult>('check_for_updates');
      updateResult.value = result;
      lastCheckedAt.value = Date.now();
      if (result.updateAvailable || showUpToDateModal) {
        showUpdateModal.value = true;
      }
      if (!result.updateAvailable && showUpToDateModal) {
        showToast('You are already on the latest version.', 'info');
      }
    } catch (error) {
      if (showUpToDateModal) {
        showToast(`Failed to check for updates: ${error}`, 'error');
      } else {
        console.log('Background update check failed:', error);
      }
    } finally {
      checkingUpdates.value = false;
    }
  };

  const openLatestReleasePage = async () => {
    const releaseUrl = updateResult.value?.releaseUrl || 'https://github.com/jackbrumley/voquill/releases/latest';
    try {
      await open(releaseUrl);
    } catch (error) {
      showToast(`Failed to open release page: ${error}`, 'error');
    }
  };

  const getLastCheckedLabel = (): string => {
    if (!lastCheckedAt.value) {
      return 'Not checked yet';
    }

    const elapsedMs = Date.now() - lastCheckedAt.value;
    if (elapsedMs < 60_000) {
      return 'Just now';
    }

    const elapsedMinutes = Math.floor(elapsedMs / 60_000);
    if (elapsedMinutes < 60) {
      return `${elapsedMinutes} min ago`;
    }

    const elapsedHours = Math.floor(elapsedMinutes / 60);
    if (elapsedHours < 24) {
      return `${elapsedHours} hr ago`;
    }

    const elapsedDays = Math.floor(elapsedHours / 24);
    return `${elapsedDays} day${elapsedDays === 1 ? '' : 's'} ago`;
  };

  return {
    updateResult: updateResult.value,
    lastCheckedAt: lastCheckedAt.value,
    checkingUpdates: checkingUpdates.value,
    showUpdateModal: showUpdateModal.value,
    setShowUpdateModal: (show) => { showUpdateModal.value = show; },
    checkForUpdates,
    openLatestReleasePage,
    getLastCheckedLabel,
  };
}