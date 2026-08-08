import { useSignal, useSignalEffect } from '@preact/signals';
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
  const { showToast } = options;
  const setupTouched = useSignal(false);
  const initialRouteChecked = useSignal(false);
  const isTestingApi = useSignal(false);

  const routeFromHash = (hash: string): AppRoute => {
    const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
    if (normalized === 'setup' || normalized === 'home' || normalized === 'history' || normalized === 'settings' || normalized === 'ui-lab') {
      return normalized;
    }
    if (normalized === 'status') {
      return 'home';
    }
    return 'home';
  };

  const hashHasExplicitRoute = (hash: string): boolean => {
    const normalized = hash.replace(/^#\/?/, '').trim().toLowerCase();
    return normalized.length > 0;
  };

  const navigate = (route: AppRoute, replace = false) => {
    const nextHash = `#/${route}`;
    if (window.location.hash === nextHash) {
      return;
    }

    if (replace) {
      window.history.replaceState(null, '', nextHash);
      return;
    }

    window.location.hash = nextHash;
  };

  useSignalEffect(() => {
    if (initialRouteChecked.value || !options.startupChecksLoaded) {
      return;
    }

    const hasExplicitRoute = hashHasExplicitRoute(window.location.hash);
    const currentHashRoute = routeFromHash(window.location.hash);

    if (currentHashRoute === 'ui-lab') {
      initialRouteChecked.value = true;
      return;
    }

    const isAllReady = options.isPortalSetupReady && options.isAudioDeviceReady && options.isLocalModelReady;

    if (isAllReady) {
      if (!hasExplicitRoute || currentHashRoute === 'setup') {
        navigate('home', true);
      }
    } else if (!hasExplicitRoute || currentHashRoute !== 'setup') {
      navigate('setup', true);
    }

    initialRouteChecked.value = true;
  });

  const handleFactoryReset = async (
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

      setupTouched.value = false;
      initialRouteChecked.value = false;
      navigate('setup', true);
    } catch (error) {
      showToast(`Factory reset failed: ${error}`, 'error');
    }
  };

  const testApiKey = async (apiKey: string, apiUrl: string) => {
    isTestingApi.value = true;
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
      isTestingApi.value = false;
    }
  };

  return {
    setupTouched: setupTouched.value,
    setSetupTouched: (touched) => { setupTouched.value = touched; },
    navigate,
    handleFactoryReset,
    testApiKey,
    isTestingApi: isTestingApi.value,
  };
}