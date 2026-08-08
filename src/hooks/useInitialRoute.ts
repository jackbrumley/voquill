import { useState, useCallback, useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import type { AppRoute } from '../types.ts';

interface UseInitialRouteOptions {
  isPortalSetupReady: boolean;
  isAudioDeviceReady: boolean;
  isLocalModelReady: boolean;
  startupChecksLoaded: boolean;
  showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

interface UseInitialRouteReturn {
  setupTouched: boolean;
  setSetupTouched: (touched: boolean) => void;
  navigate: (route: AppRoute, replace?: boolean) => void;
  handleFactoryReset: (loadConfig: () => Promise<void>, loadMics: () => Promise<void>, loadModels: () => Promise<void>, loadHistory: () => Promise<void>, checkSetupStatus: () => Promise<void>) => Promise<void>;
  testApiKey: (apiKey: string, apiUrl: string) => Promise<void>;
  isTestingApi: boolean;
}

export function useInitialRoute(options: UseInitialRouteOptions): UseInitialRouteReturn {
  const { isPortalSetupReady, isAudioDeviceReady, isLocalModelReady, startupChecksLoaded, showToast } = options;
  const [setupTouched, setSetupTouched] = useState(false);
  const [initialRouteChecked, setInitialRouteChecked] = useState(false);
  const [isTestingApi, setIsTestingApi] = useState(false);

  const isAllReady = isPortalSetupReady && isAudioDeviceReady && isLocalModelReady;

  const routeFromHash = useCallback((hash: string): AppRoute => {
    const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
    if (normalized === 'setup' || normalized === 'status' || normalized === 'history' || normalized === 'settings' || normalized === 'ui-lab') {
      return normalized;
    }
    return 'status';
  }, []);

  const hashHasExplicitRoute = useCallback((hash: string): boolean => {
    const normalized = hash.replace(/^#\/?/, '').trim().toLowerCase();
    return normalized.length > 0;
  }, []);

  const navigate = useCallback((route: AppRoute, replace = false) => {
    const nextHash = `#/${route}`;
    if (window.location.hash === nextHash) {
      return;
    }

    if (replace) {
      window.history.replaceState(null, '', nextHash);
      return;
    }

    window.location.hash = nextHash;
  }, []);

  // Initial route redirection
  useEffect(() => {
    if (initialRouteChecked || !startupChecksLoaded) {
      return;
    }

    const hasExplicitRoute = hashHasExplicitRoute(window.location.hash);
    const currentHashRoute = routeFromHash(window.location.hash);

    if (currentHashRoute === 'ui-lab') {
      setInitialRouteChecked(true);
      return;
    }

    if (isAllReady) {
      if (!hasExplicitRoute || currentHashRoute === 'setup') {
        navigate('status', true);
      }
    } else if (!hasExplicitRoute || currentHashRoute !== 'setup') {
      navigate('setup', true);
    }

    setInitialRouteChecked(true);
  }, [initialRouteChecked, startupChecksLoaded, isAllReady, navigate, routeFromHash, hashHasExplicitRoute]);

  const handleFactoryReset = useCallback(async (
    loadConfig: () => Promise<void>,
    loadMics: () => Promise<void>,
    loadModels: () => Promise<void>,
    loadHistory: () => Promise<void>,
    checkSetupStatus: () => Promise<void>,
  ) => {
    try {
      await invoke('reset_application_to_defaults');
      showToast('Factory reset completed.', 'success');

      await Promise.all([
        loadConfig(),
        loadMics(),
        loadModels(),
        loadHistory(),
        checkSetupStatus(),
      ]);

      setSetupTouched(false);
      setInitialRouteChecked(false);
      navigate('setup', true);
    } catch (error) {
      showToast(`Factory reset failed: ${error}`, 'error');
    }
  }, [showToast, navigate]);

  const testApiKey = useCallback(async (apiKey: string, apiUrl: string) => {
    setIsTestingApi(true);
    try {
      const isValid = await invoke<boolean>('test_api_key', { apiKey, apiUrl });
      if (isValid) {
        showToast('API Key is valid!', 'success');
      } else {
        showToast('API Key is invalid or rate limited.', 'error');
      }
    } catch (error) {
      showToast(`API Test Failed: ${error}`, 'error');
    } finally {
      setIsTestingApi(false);
    }
  }, [showToast]);

  return {
    setupTouched,
    setSetupTouched,
    navigate,
    handleFactoryReset,
    testApiKey,
    isTestingApi,
  };
}