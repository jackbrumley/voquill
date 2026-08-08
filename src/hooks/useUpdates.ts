import { useState, useCallback } from 'preact/hooks';
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
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [lastCheckedAt, setLastCheckedAt] = useState<number | null>(null);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [showUpdateModal, setShowUpdateModal] = useState(false);

  const checkForUpdates = useCallback(async (showUpToDateModal: boolean) => {
    if (checkingUpdates) return;

    setCheckingUpdates(true);
    try {
      const result = await invoke<UpdateCheckResult>('check_for_updates');
      setUpdateResult(result);
      setLastCheckedAt(Date.now());
      if (result.updateAvailable || showUpToDateModal) {
        setShowUpdateModal(true);
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
      setCheckingUpdates(false);
    }
  }, [checkingUpdates, showToast]);

  const openLatestReleasePage = useCallback(async () => {
    const releaseUrl = updateResult?.releaseUrl || 'https://github.com/jackbrumley/voquill/releases/latest';
    try {
      await open(releaseUrl);
    } catch (error) {
      showToast(`Failed to open release page: ${error}`, 'error');
    }
  }, [updateResult, showToast]);

  const getLastCheckedLabel = useCallback((): string => {
    if (!lastCheckedAt) {
      return 'Not checked yet';
    }

    const elapsedMs = Date.now() - lastCheckedAt;
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
  }, [lastCheckedAt]);

  return {
    updateResult,
    lastCheckedAt,
    checkingUpdates,
    showUpdateModal,
    setShowUpdateModal,
    checkForUpdates,
    openLatestReleasePage,
    getLastCheckedLabel,
  };
}