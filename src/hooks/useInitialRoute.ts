import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { AppRoute } from '../types.ts';
import { explainReadiness, type ReadinessInputs, type ReadinessStatus } from '../readiness.ts';

interface UseInitialRouteOptions {
  /// Signal (not a plain value) so the routing effect is retriggered when
  /// startup probes finish. The readiness snapshot stays a plain value:
  /// useSignalEffect always invokes the latest render's closure, so the
  /// decision reads fresh data once the signal flips.
  startupChecksLoaded: { readonly value: boolean };
  /// The app's active-route signal. navigate() writes it directly because
  /// replaceState (used for redirects) fires no hashchange event.
  activeRoute: { value: AppRoute };
  readiness: ReadinessStatus;
  readinessInputs: ReadinessInputs;
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

  /// Mid-session readiness gate: navigating to Home while the app cannot
  /// transcribe (permissions, mic, model/key, hotkey) lands on setup instead.
  /// Runs in event-handler context, so signal/plain reads are always fresh.
  /// Only rewrites 'home' -> 'setup', so it can never redirect-loop.
  const guardHomeNavigation = (route: AppRoute): AppRoute => {
    if (route === 'home' && options.startupChecksLoaded.value && !options.readiness.isAllReady) {
      const explanation = explainReadiness(options.readinessInputs, options.readiness);
      const msg = `[Navigation Guard] Blocked navigation to 'home' -> redirected to 'setup'. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      return 'setup';
    }
    return route;
  };

  const navigate = (route: AppRoute, replace = false) => {
    const guardedRoute = guardHomeNavigation(route);
    const explanation = explainReadiness(options.readinessInputs, options.readiness);
    const logMsg = `[Navigation] Requested route '${route}' -> target '${guardedRoute}' (replace=${replace}). ${explanation}`;
    console.log(logMsg);
    invoke('log_ui_event', { message: logMsg }).catch(() => {});

    const nextHash = `#/${guardedRoute}`;
    // Update the route signal directly: replaceState fires no hashchange
    // event, so redirects would otherwise never reach the view.
    options.activeRoute.value = guardedRoute;
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
    if (initialRouteChecked.value || !options.startupChecksLoaded.value) {
      return;
    }

    const hasExplicitRoute = hashHasExplicitRoute(window.location.hash);
    const currentHashRoute = routeFromHash(window.location.hash);
    const explanation = explainReadiness(options.readinessInputs, options.readiness);

    if (currentHashRoute === 'ui-lab') {
      const msg = `[Initial Route] Startup checks loaded. Route is 'ui-lab'; bypassing setup check. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      initialRouteChecked.value = true;
      return;
    }

    if (options.readiness.isAllReady) {
      if (!hasExplicitRoute || currentHashRoute === 'setup') {
        const msg = `[Initial Route] Startup checks loaded. All checks ready -> routing to 'home'. ${explanation}`;
        console.log(msg);
        invoke('log_ui_event', { message: msg }).catch(() => {});
        navigate('home', true);
      } else {
        const msg = `[Initial Route] Startup checks loaded. All checks ready; preserving explicit route '${currentHashRoute}'. ${explanation}`;
        console.log(msg);
        invoke('log_ui_event', { message: msg }).catch(() => {});
      }
    } else if (!hasExplicitRoute || currentHashRoute !== 'setup') {
      const msg = `[Initial Route] Startup checks loaded. Not ready -> routing to 'setup'. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      navigate('setup', true);
    } else {
      const msg = `[Initial Route] Startup checks loaded. Not ready; already on 'setup'. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
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