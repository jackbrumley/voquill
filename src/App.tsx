import { useEffect, useRef } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { TitleBar } from './components/TitleBar.tsx';
import { Modals } from './components/Modals.tsx';
import { MainLayout } from './components/MainLayout.tsx';
import { InitialSetupPage } from './pages/InitialSetupPage.tsx';
import { appShellStyle, appContentStyle, resizeCornerOverlayStyle, resizeCornerStyles } from './theme/ui-primitives.ts';
import { useToast } from './hooks/useToast.tsx';
import { useTauriEvents } from './hooks/useTauriEvents.ts';
import { useConfig } from './hooks/useConfig.ts';
import { useAudioSetup } from './hooks/useAudioSetup.ts';
import { useHotkeySetup } from './hooks/useHotkeySetup.ts';
import { useHistory } from './hooks/useHistory.ts';
import { useUpdates } from './hooks/useUpdates.ts';
import { useAutostart } from './hooks/useAutostart.ts';
import { useWindowControls } from './hooks/useWindowControls.ts';
import { useInitialRoute } from './hooks/useInitialRoute.ts';
import { useGpuStatus } from './hooks/useGpuStatus.ts';
import { computeReadiness } from './readiness.ts';
import type { AppRoute } from './types.ts';

function App() {
  const { showToast, ToastContainer } = useToast();

  const logUI = (msg: string) => {
    if (
      !msg.includes('Button clicked') &&
      !msg.includes('Toast') &&
      !msg.includes('Setting changed') &&
      !msg.includes('Switch toggled')
    ) {
      return;
    }
    const timestamp = new Date().toLocaleTimeString();
    console.log(`[${timestamp}] ${msg}`);
    invoke('log_ui_event', { message: msg }).catch((err) => {
      console.error(`Failed to send log to backend: ${err}`);
    });
  };

  const configHook = useConfig(showToast, logUI);
  const audioSetup = useAudioSetup(showToast);
  const historyHook = useHistory(showToast);
  const updatesHook = useUpdates(showToast);
  const autostartHook = useAutostart(showToast);
  const windowControls = useWindowControls(showToast);

  const showModelGuide = useSignal(false);
  const showPostProcessGuide = useSignal(false);
  const activeConfigSection = useSignal<string | null>(null);
  const appVersion = useSignal<string>('');
  const tabContentRef = useRef<HTMLDivElement | null>(null);

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

  const activeRoute = useSignal<AppRoute>(routeFromHash(window.location.hash));

  const gpuHook = useGpuStatus();
  const startupChecksLoaded = useSignal(false);

  const hotkeySetup = useHotkeySetup({
    showToast,
    onApplyCapturedHotkey: async (normalized) => {
      hotkeySetup.setIsApplyingHotkey(true);
      configHook.updateConfig('hotkey', normalized);
      try {
        await invoke('apply_captured_hotkey', { newHotkey: normalized });
        showToast('Shortcut configured successfully!', 'success');
        // Registration clears the backend hotkey error; refresh so the
        // readiness gate (and setup row) reflects it immediately.
        void audioSetup.checkSetupStatus();
      } catch (error) {
        showToast(`Failed to apply captured shortcut: ${error}`, 'error');
      } finally {
        await hotkeySetup.setRecordingState(false);
        hotkeySetup.setShowHotkeyCaptureModal(false);
        hotkeySetup.setIsApplyingHotkey(false);
      }
    },
  });

  const readiness = computeReadiness({
    permissions: audioSetup.permissions,
    hotkeyError: audioSetup.hotkeyError,
    availableMics: audioSetup.availableMics,
    config: configHook.config,
    availableModels: configHook.availableModels,
    modelStatus: configHook.modelStatus,
  });

  const {
    setupTouched,
    setSetupTouched,
    navigate,
    handleFactoryReset,
    testApiKey,
    isTestingApi,
  } = useInitialRoute({
    startupChecksLoaded,
    activeRoute,
    readiness,
    showToast,
  });

  useEffect(() => {
    void Promise.allSettled([
      configHook.loadConfig(),
      audioSetup.loadMics(),
      historyHook.loadHistory(),
      configHook.loadModels(),
      audioSetup.checkSetupStatus(),
    ]).then(() => { startupChecksLoaded.value = true; });
    getVersion().then((v) => { appVersion.value = v; }).catch(err => console.error("Failed to get version:", err));
    updatesHook.checkForUpdates(false);
    autostartHook.loadAutostart();
    void gpuHook.refreshGpuStatus();
  }, []);

  // Post-process GPU availability is refetched when a warm-up attempt
  // finishes (see onPostProcessGpuStatusChanged below): warm-ups fire on
  // startup, on engine selection, and on model download completion, which
  // covers every path that can change the result.

  useEffect(() => {
    if (!hotkeySetup.isRecordingHotkey) return;

    window.addEventListener('keydown', hotkeySetup.handleHotkeyKeyDown);
    window.addEventListener('keyup', hotkeySetup.handleHotkeyKeyUp);

    return () => {
      window.removeEventListener('keydown', hotkeySetup.handleHotkeyKeyDown);
      window.removeEventListener('keyup', hotkeySetup.handleHotkeyKeyUp);
    };
  }, [hotkeySetup.isRecordingHotkey]);

  useEffect(() => {
    activeConfigSection.value = null;
    if (tabContentRef.current) {
      tabContentRef.current.scrollTop = 0;
    }
  }, [activeRoute.value]);

  useTauriEvents({
    onSetupStatus: (payload) => {
      if (payload === 'configuring-system') {
        showToast('Configuring system permissions...', 'info');
      } else if (payload === 'restart-required') {
        showToast('Permissions updated! Please restart your session.', 'success');
      } else if (payload === 'setup-failed') {
        showToast('System configuration failed.', 'error');
      }
    },
    onStatusUpdate: (payload) => {
      const nextStatus = typeof payload === 'string' ? payload : payload.status;
      if (nextStatus === 'Error') {
        showToast('Mic not found — check your audio device settings.', 'error');
      }
    },
    onHistoryUpdated: () => { historyHook.loadHistory(); },
    onConfigUpdated: () => { configHook.loadConfig(); },
    onHotkeyBindingState: hotkeySetup.setHotkeyBindingState,
    onMicTestStarted: () => { audioSetup.setMicTestStatus('playing'); },
    onMicTestFinished: () => {
      audioSetup.setMicTestStatus('idle');
      audioSetup.setMicVolume(0);
      audioSetup.setMicTestPassed(true);
    },
    onMicVolume: audioSetup.setMicVolume,
    onDownloadProgress: (progress) => { configHook.setDownloadProgress(progress); },
    onPostProcessGpuStatusChanged: () => { void gpuHook.refreshPostProcessGpuStatus(); },
    // Focus is the natural re-probe boundary: external changes that affect
    // readiness (models deleted, mic unplugged, permissions revoked) happen
    // while the app is unfocused.
    onFocus: () => {
      void audioSetup.checkSetupStatus();
      void audioSetup.loadMics();
      void configHook.loadModels();
    },
    onHashChange: () => {
      // Funnel hash edits/back-forward through the same guard as in-app
      // navigation; replace keeps the guarded URL out of history.
      navigate(routeFromHash(window.location.hash), true);
    },
  });

  const handleSetActiveConfigSection = (value: string | null) => {
    activeConfigSection.value = value;
    if (tabContentRef.current) {
      tabContentRef.current.scrollTop = 0;
    }
  };

  return (
    <div style={appShellStyle}>
      <TitleBar
        onMinimize={windowControls.handleMinimize}
        onMaximize={() => void windowControls.toggleWindowMaximize()}
        onClose={windowControls.handleClose}
        onMouseDown={windowControls.handleTitleBarMouseDown}
        onDoubleClick={windowControls.handleTitleBarDoubleClick}
      />

      {activeRoute.value === 'setup' ? (
        <div style={appContentStyle}>
          <InitialSetupPage
            permissions={audioSetup.permissions}
            config={configHook.config}
            readiness={readiness}
            availableEngines={configHook.availableEngines}
            availableModels={configHook.availableModels}
            modelStatus={configHook.modelStatus}
            downloadProgress={configHook.downloadProgress}
            downloadPhase={configHook.downloadPhase}
            isDownloading={configHook.isDownloading}
            portalVersion={hotkeySetup.portalVersion}
            isSystemManagedShortcut={hotkeySetup.isSystemManagedShortcut}
            systemShortcutContext={hotkeySetup.systemShortcutContext}
            isApplyingHotkey={hotkeySetup.isApplyingHotkey}
            hotkeyError={audioSetup.hotkeyError}
            availableMics={audioSetup.availableMics}
            micTestStatus={audioSetup.micTestStatus}
            micVolume={audioSetup.micVolume}
            micTestPassed={audioSetup.micTestPassed}
            gpuStatus={gpuHook.gpuStatus}
            isTestingEngine={gpuHook.isTestingEngine}
            isTestingApi={isTestingApi}
            isRecordingHotkey={hotkeySetup.isRecordingHotkey}
            setupTouched={setupTouched}
            onTouchSetup={() => setSetupTouched(true)}
            onAudioSetup={() => void audioSetup.handleAudioSetup()}
            onInputSetup={() => void audioSetup.handleInputSetup()}
            onConfigureHotkey={() => void hotkeySetup.handleConfigureHotkey()}
            onHotkeyKeyDown={hotkeySetup.handleHotkeyKeyDown}
            onHotkeyKeyUp={hotkeySetup.handleHotkeyKeyUp}
            onHotkeyBlur={() => void hotkeySetup.setRecordingState(false)}
            onChangeConfig={configHook.updateConfig}
            onSelectEngine={(engine) => {
              setSetupTouched(true);
              configHook.updateConfig('local_engine', engine);
              if (engine.includes('(GPU)')) {
                void gpuHook.testTranscriptionEngine();
              }
            }}
            onShowModelGuide={() => { showModelGuide.value = true; }}
            onDownloadModel={(size) => {
              void configHook.downloadModel(size).then(() => {
                if (configHook.config.local_engine.includes('(GPU)')) {
                  void gpuHook.testTranscriptionEngine();
                }
              });
            }}
            onDownloadPostProcessModel={(size) => void configHook.downloadModel(size, configHook.config.post_process_engine)}
            onRetryModels={() => void configHook.loadModels()}
            onLoadMics={() => void audioSetup.loadMics()}
            onStartMicTest={() => void audioSetup.startMicTest()}
            onStopMicTest={() => void audioSetup.stopMicTest()}
            onStopMicPlayback={() => void audioSetup.stopMicPlayback()}
            onRefreshStatus={() => void audioSetup.checkSetupStatus()}
            onTestApiKey={() => void testApiKey(configHook.config.openai_api_key, configHook.config.api_url)}
            onFinishSetup={() => navigate('home')}
          />
        </div>
      ) : (
        <MainLayout
          activeRoute={activeRoute.value}
          config={configHook.config}
          appVersion={appVersion.value}
          availableEngines={configHook.availableEngines}
          availableModels={configHook.availableModels}
          modelStatus={configHook.modelStatus}
          downloadProgress={configHook.downloadProgress}
          downloadPhase={configHook.downloadPhase}
          isDownloading={configHook.isDownloading}
          isTestingApi={isTestingApi}
          activeConfigSection={activeConfigSection.value}
          portalVersion={hotkeySetup.portalVersion}
          isSystemManagedShortcut={hotkeySetup.isSystemManagedShortcut}
          hotkeyBindingState={hotkeySetup.hotkeyBindingState}
          isApplyingHotkey={hotkeySetup.isApplyingHotkey}
          availableMics={audioSetup.availableMics}
          micTestStatus={audioSetup.micTestStatus}
          micVolume={audioSetup.micVolume}
          overlayPositioningCapabilities={hotkeySetup.overlayPositioningCapabilities}
          checkingUpdates={updatesHook.checkingUpdates}
          autostartEnabled={autostartHook.autostartEnabled}
          history={historyHook.history}
          searchQuery={historyHook.searchQuery}
          searchResults={historyHook.searchResults}
          updateResult={updatesHook.updateResult}
          gpuStatus={gpuHook.gpuStatus}
          postProcessGpuStatus={gpuHook.postProcessGpuStatus}
          engineCapabilities={configHook.engineCapabilities}
          tabContentRef={tabContentRef}
          onNavigate={navigate}
          onLogUI={logUI}
          onSetActiveConfigSection={handleSetActiveConfigSection}
          onUpdateConfig={configHook.updateConfig}
          onTestApiKey={() => void testApiKey(configHook.config.openai_api_key, configHook.config.api_url)}
          onDownloadModel={configHook.downloadModel}
          onLoadModels={configHook.loadModels}
          onLoadMics={audioSetup.loadMics}
          onHandleConfigureHotkey={hotkeySetup.handleConfigureHotkey}
          onSetShowModelGuide={(v) => { showModelGuide.value = v; }}
          onSetShowPostProcessGuide={(v) => { showPostProcessGuide.value = v; }}
          onStartMicTest={audioSetup.startMicTest}
          onStopMicTest={audioSetup.stopMicTest}
          onStopMicPlayback={audioSetup.stopMicPlayback}
          onOpenDebugFolder={() => void invoke('open_debug_folder').catch(() => showToast('Failed to open debug folder', 'error'))}
          onReopenInitialSetup={() => { setSetupTouched(true); navigate('setup'); }}
          onFactoryReset={() => hotkeySetup.setShowFactoryResetModal(true)}
          onCheckForUpdates={() => void updatesHook.checkForUpdates(true)}
          onOpenUiLab={() => navigate('ui-lab')}
          onToggleAutostart={autostartHook.toggleAutostart}
          onCopyToClipboard={historyHook.copyToClipboard}
          onClearHistory={historyHook.clearHistory}
          onSearchHistory={historyHook.setSearchQuery}
          onTestCleanupApi={() => {
            const cfg = configHook.config;
            invoke('test_cleanup_api', {
              apiKey: cfg.post_process_api_key,
              apiUrl: cfg.post_process_api_url,
              model: cfg.post_process_api_model,
              systemPrompt: cfg.post_process_prompt,
            })
              .then((result) => showToast(`Post-processing test: ${result}`, 'success'))
              .catch((err) => showToast(`Post-processing test failed: ${err}`, 'error'));
          }}
          onToggleOutputMethod={configHook.toggleOutputMethod}
          onOpenUpdateModal={() => updatesHook.setShowUpdateModal(true)}
        />
      )}

      <div style={resizeCornerOverlayStyle}>
        <div style={resizeCornerStyles.nw}
          onMouseDown={windowControls.handleResizeCornerMouseDown('NorthWest')} />
        <div style={resizeCornerStyles.ne}
          onMouseDown={windowControls.handleResizeCornerMouseDown('NorthEast')} />
        <div style={resizeCornerStyles.sw}
          onMouseDown={windowControls.handleResizeCornerMouseDown('SouthWest')} />
        <div style={resizeCornerStyles.se}
          onMouseDown={windowControls.handleResizeCornerMouseDown('SouthEast')} />
      </div>

      <ToastContainer />

      <Modals
        showHotkeyCaptureModal={hotkeySetup.showHotkeyCaptureModal}
        showSystemShortcutModal={hotkeySetup.showSystemShortcutModal}
        showFactoryResetModal={hotkeySetup.showFactoryResetModal}
        showUpdateModal={updatesHook.showUpdateModal}
        showModelGuide={showModelGuide.value}
        showPostProcessGuide={showPostProcessGuide.value}
        isRecordingHotkey={hotkeySetup.isRecordingHotkey}
        isApplyingHotkey={hotkeySetup.isApplyingHotkey}
        configHotkey={configHook.config.hotkey}
        systemShortcutContext={hotkeySetup.systemShortcutContext}
        hotkeyBindingState={hotkeySetup.hotkeyBindingState}
        updateResult={updatesHook.updateResult}
        appVersion={appVersion.value}
        getLastCheckedLabel={updatesHook.getLastCheckedLabel}
        onCancelHotkeyCapture={() => void hotkeySetup.cancelHotkeyCapture()}
        onCloseSystemShortcut={() => hotkeySetup.setShowSystemShortcutModal(false)}
        onChangedSystemShortcut={() => {
          hotkeySetup.setShowSystemShortcutModal(false);
          void audioSetup.checkSetupStatus();
          void configHook.loadConfig();
        }}
        onCloseFactoryReset={() => hotkeySetup.setShowFactoryResetModal(false)}
        onFactoryReset={() => void handleFactoryReset(configHook.loadConfig, audioSetup.loadMics, configHook.loadModels, historyHook.loadHistory, () => audioSetup.checkSetupStatus().then(() => {}))}
        onCloseUpdate={() => updatesHook.setShowUpdateModal(false)}
        onOpenLatestRelease={() => void updatesHook.openLatestReleasePage()}
        onCloseModelGuide={() => { showModelGuide.value = false; }}
        onClosePostProcessGuide={() => { showPostProcessGuide.value = false; }}
      />
    </div>
  );
}

export default App;