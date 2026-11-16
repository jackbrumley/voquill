import { useSignal, useSignalEffect } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { AppRoute } from '../types.ts';
import type { SetupRecommendation, SetupReview } from '../pages/setup/setupTypes.ts';
import { describeReadiness, type ReadinessStatus } from '../readiness.ts';

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
  showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

interface UseInitialRouteReturn {
  /// Set when setup opened as a review of new options for an existing user.
  setupReview: SetupReview | null;
  completeSetup: () => Promise<void>;
  previewSetupReview: () => Promise<void>;
  navigate: (route: AppRoute, replace?: boolean) => void;
  handleFactoryReset: (loadConfig: () => Promise<void>, loadMics: () => Promise<void>, loadModels: () => Promise<void>, loadHistory: () => Promise<void>, checkSetupStatus: () => Promise<void>) => Promise<void>;
  testApiKey: (apiKey: string, apiUrl: string) => Promise<void>;
  isTestingApi: boolean;
}

export function useInitialRoute(options: UseInitialRouteOptions): UseInitialRouteReturn {
  const { showToast } = options;
  const initialRouteChecked = useSignal(false);
  const isTestingApi = useSignal(false);
  const setupReview = useSignal<SetupReview | null>(null);

  const routeFromHash = (hash: string): AppRoute => {
    const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
    if (normalized === 'setup' || normalized === 'home' || normalized === 'history' || normalized === 'settings' || normalized === 'ui-lab' || normalized === 'help') {
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
      const explanation = describeReadiness(options.readiness);
      const msg = `[Navigation Guard] Blocked navigation to 'home' -> redirected to 'setup'. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      return 'setup';
    }
    return route;
  };

  const navigate = (route: AppRoute, replace = false) => {
    const guardedRoute = guardHomeNavigation(route);
    const explanation = describeReadiness(options.readiness);
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

  /// A ready app normally opens Home, but after an update that bumped the
  /// setup revision (the backend decides, see `setup_review`), setup opens
  /// as a review until it is finished or kept. The app stays fully usable.
  const routeReadyLaunch = async (hasExplicitRoute: boolean, currentHashRoute: AppRoute, explanation: string) => {
    let review: SetupReview = { due: false, recommended: null, already_recommended: false };
    try {
      review = await invoke<SetupReview>('get_setup_review');
    } catch (error) {
      const msg = `[Initial Route] Setup review check failed; continuing to the normal route: ${error}`;
      console.error(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
    }

    if (review.due) {
      setupReview.value = review;
      const msg = `[Initial Route] Startup checks loaded. All checks ready; setup has new options -> routing to 'setup' for review. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      navigate('setup', true);
    } else if (!hasExplicitRoute || currentHashRoute === 'setup') {
      const msg = `[Initial Route] Startup checks loaded. All checks ready -> routing to 'home'. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      navigate('home', true);
    } else {
      const msg = `[Initial Route] Startup checks loaded. All checks ready; preserving explicit route '${currentHashRoute}'. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
    }
  };

  /// Debug: opens setup as the after-update review with this computer's
  /// real recommendation, whether or not a review is actually due. Previews
  /// the "better choice" variant of the review.
  const previewSetupReview = async () => {
    try {
      const recommendation = await invoke<SetupRecommendation>('get_setup_recommendation');
      setupReview.value = { due: true, recommended: recommendation.options[0] ?? null, already_recommended: false };
      navigate('setup');
    } catch (error) {
      showToast(`Couldn't load the setup recommendation: ${error}`, 'error');
    }
  };

  /// Finishing or skipping setup records the current setup revision, so a
  /// review is never offered again until setup changes.
  const completeSetup = async () => {
    try {
      await invoke('complete_setup_review');
    } catch (error) {
      showToast(`Couldn't save setup progress: ${error}`, 'error');
    }
    setupReview.value = null;
    navigate('home');
  };

  useSignalEffect(() => {
    if (initialRouteChecked.value || !options.startupChecksLoaded.value) {
      return;
    }

    const hasExplicitRoute = hashHasExplicitRoute(window.location.hash);
    const currentHashRoute = routeFromHash(window.location.hash);
    const explanation = describeReadiness(options.readiness);

    if (currentHashRoute === 'ui-lab') {
      const msg = `[Initial Route] Startup checks loaded. Route is 'ui-lab'; bypassing setup check. ${explanation}`;
      console.log(msg);
      invoke('log_ui_event', { message: msg }).catch(() => {});
      initialRouteChecked.value = true;
      return;
    }

    if (options.readiness.isAllReady) {
      initialRouteChecked.value = true;
      void routeReadyLaunch(hasExplicitRoute, currentHashRoute, explanation);
      return;
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
    navigate,
    setupReview: setupReview.value,
    completeSetup,
    previewSetupReview,
    handleFactoryReset,
    testApiKey,
    isTestingApi: isTestingApi.value,
  };
}