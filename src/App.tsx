import { useState, useEffect, useRef, useCallback } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { TitleBar } from './components/TitleBar.tsx';
import { Modals } from './components/Modals.tsx';
import { MainLayout } from './components/MainLayout.tsx';
import { InitialSetupPage } from './pages/InitialSetupPage.tsx';
import { appShellStyle, appContentStyle } from './theme/ui-primitives.ts';
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
import type { AppRoute } from './types.ts';

function App() {
  const { showToast, ToastContainer } = useToast();

  const logUIRef = useRef<(msg: string) => void>(() => {});
  const logUI = useCallback((msg: string) => {
    logUIRef.current(msg);
  }, []);

  const configHook = useConfig(showToast, logUI);
  logUIRef.current = (msg: string) => {
    if (
      !configHook.config.debug_mode &&
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
  const audioSetup = useAudioSetup(showToast);
  const historyHook = useHistory(showToast);
  const updatesHook = useUpdates(showToast);
  const autostartHook = useAutostart(showToast);
  const windowControls = useWindowControls(showToast);

  const [activeRoute] = useState<AppRoute>(() => {
    const hash = window.location.hash;
    const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
    if (normalized === 'setup' || normalized === 'status' || normalized === 'history' || normalized === 'settings' || normalized === 'ui-lab') {
      return normalized;
    }
    return 'status';
  });
  const [currentStatus, setCurrentStatus] = useState<string>('Ready');
  const [showModelGuide, setShowModelGuide] = useState(false);
  const [activeConfigSection, setActiveConfigSection] = useState<string | null>(null);
  const [appVersion, setAppVersion] = useState<string>('');
  const [hoveredTopTab, setHoveredTopTab] = useState<AppRoute | null>(null);
  const tabContentRef = useRef<HTMLDivElement | null>(null);

  const routeFromHash = (hash: string): AppRoute => {
    const normalized = hash.replace(/^#\/?/, '').split('/')[0].trim().toLowerCase();
    if (normalized === 'setup' || normalized === 'status' || normalized === 'history' || normalized === 'settings' || normalized === 'ui-lab') {
      return normalized;
    }
    return 'status';
  };

  const hotkeySetup = useHotkeySetup({
    showToast,
    onApplyCapturedHotkey: async (normalized) => {
      hotkeySetup.setIsApplyingHotkey(true);
      configHook.updateConfig('hotkey', normalized);
      try {
        await invoke('apply_captured_hotkey', { newHotkey: normalized });
        showToast('Shortcut configured successfully!', 'success');
      } catch (error) {
        showToast(`Failed to apply captured shortcut: ${error}`, 'error');
      } finally {
        await hotkeySetup.setRecordingState(false);
        hotkeySetup.setShowHotkeyCaptureModal(false);
        hotkeySetup.setIsApplyingHotkey(false);
      }
    },
  });

  const {
    setupTouched,
    setSetupTouched,
    navigate,
    handleFactoryReset,
    testApiKey,
    isTestingApi,
  } = useInitialRoute({
    isPortalSetupReady: audioSetup.permissions
      ? audioSetup.permissions.audio && audioSetup.permissions.shortcuts && audioSetup.permissions.input_emulation
      : false,
    isAudioDeviceReady: audioSetup.availableMics.length > 0 && !!configHook.config.audio_device,
    isLocalModelReady: configHook.config.transcription_mode !== 'Local' || !!configHook.modelStatus[configHook.config.local_model_size],
    startupChecksLoaded: configHook.hasLoadedConfig && audioSetup.hasLoadedSetupStatus && audioSetup.hasLoadedMics && configHook.hasLoadedModels,
    showToast,
  });

  // Initialize app data on mount
  useEffect(() => {
    configHook.loadConfig();
    audioSetup.loadMics();
    historyHook.loadHistory();
    configHook.loadModels();
    audioSetup.checkSetupStatus();
    getVersion().then(setAppVersion).catch(err => console.error("Failed to get version:", err));
    updatesHook.checkForUpdates(false);
    autostartHook.loadAutostart();
  }, []);

  // Hotkey recording keydown/keyup listeners
  useEffect(() => {
    if (!hotkeySetup.isRecordingHotkey) return;

    window.addEventListener('keydown', hotkeySetup.handleHotkeyKeyDown);
    window.addEventListener('keyup', hotkeySetup.handleHotkeyKeyUp);

    return () => {
      window.removeEventListener('keydown', hotkeySetup.handleHotkeyKeyDown);
      window.removeEventListener('keyup', hotkeySetup.handleHotkeyKeyUp);
    };
  }, [hotkeySetup.isRecordingHotkey, hotkeySetup.recordedKeys]);

  // Scroll tab content to top on route change
  useEffect(() => {
    if (tabContentRef.current) {
      tabContentRef.current.scrollTop = 0;
    }
  }, [activeRoute]);

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
      setCurrentStatus(nextStatus);
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
    onDownloadProgress: (_progress) => {},
    onFocus: () => { audioSetup.checkSetupStatus(); },
    onHashChange: () => {
      navigate(routeFromHash(window.location.hash), true);
    },
  });

  const handleSetActiveConfigSection = (value: string | null) => {
    setActiveConfigSection(value);
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

      {activeRoute === 'setup' ? (
        <div style={appContentStyle}>
          <InitialSetupPage
            permissions={audioSetup.permissions}
            config={configHook.config}
            availableModels={configHook.availableModels}
            modelStatus={configHook.modelStatus}
            downloadProgress={configHook.downloadProgress}
            isDownloading={configHook.isDownloading}
            portalVersion={hotkeySetup.portalVersion}
            isSystemManagedShortcut={hotkeySetup.isSystemManagedShortcut}
            systemShortcutContext={hotkeySetup.systemShortcutContext}
            isApplyingHotkey={hotkeySetup.isApplyingHotkey}
            availableMics={audioSetup.availableMics}
            micTestStatus={audioSetup.micTestStatus}
            micVolume={audioSetup.micVolume}
            micTestPassed={audioSetup.micTestPassed}
            isLocalModelReady={configHook.config.transcription_mode !== 'Local' || !!configHook.modelStatus[configHook.config.local_model_size]}
            isAudioDeviceReady={audioSetup.availableMics.length > 0 && !!configHook.config.audio_device}
            isAllReady={!!(audioSetup.permissions?.audio && audioSetup.permissions?.shortcuts && audioSetup.permissions?.input_emulation) && audioSetup.availableMics.length > 0 && !!configHook.config.audio_device && (configHook.config.transcription_mode !== 'Local' || !!configHook.modelStatus[configHook.config.local_model_size])}
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
            onShowModelGuide={() => setShowModelGuide(true)}
            onDownloadModel={(size) => void configHook.downloadModel(size)}
            onRetryModels={() => void configHook.loadModels()}
            onLoadMics={() => void audioSetup.loadMics()}
            onStartMicTest={() => void audioSetup.startMicTest()}
            onStopMicTest={() => void audioSetup.stopMicTest()}
            onStopMicPlayback={() => void audioSetup.stopMicPlayback()}
            onRefreshStatus={() => void audioSetup.checkSetupStatus()}
            onFinishSetup={() => navigate('status')}
          />
        </div>
      ) : (
        <MainLayout
          activeRoute={activeRoute}
          config={configHook.config}
          currentStatus={currentStatus}
          appVersion={appVersion}
          availableEngines={configHook.availableEngines}
          availableModels={configHook.availableModels}
          modelStatus={configHook.modelStatus}
          downloadProgress={configHook.downloadProgress}
          isDownloading={configHook.isDownloading}
          isTestingApi={isTestingApi}
          activeConfigSection={activeConfigSection}
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
          hoveredTopTab={hoveredTopTab}
          history={historyHook.history}
          updateResult={updatesHook.updateResult}
          tabContentRef={tabContentRef}
          onNavigate={navigate}
          onLogUI={logUI}
          onSetHoveredTab={setHoveredTopTab}
          onSetActiveConfigSection={handleSetActiveConfigSection}
          onUpdateConfig={configHook.updateConfig}
          onTestApiKey={() => void testApiKey(configHook.config.openai_api_key, configHook.config.api_url)}
          onDownloadModel={configHook.downloadModel}
          onLoadModels={configHook.loadModels}
          onLoadMics={audioSetup.loadMics}
          onHandleConfigureHotkey={hotkeySetup.handleConfigureHotkey}
          onSetShowModelGuide={setShowModelGuide}
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
          onToggleOutputMethod={configHook.toggleOutputMethod}
          onOpenUpdateModal={() => updatesHook.setShowUpdateModal(true)}
        />
      )}

      <ToastContainer />

      <Modals
        showHotkeyCaptureModal={hotkeySetup.showHotkeyCaptureModal}
        showSystemShortcutModal={hotkeySetup.showSystemShortcutModal}
        showFactoryResetModal={hotkeySetup.showFactoryResetModal}
        showUpdateModal={updatesHook.showUpdateModal}
        showModelGuide={showModelGuide}
        isRecordingHotkey={hotkeySetup.isRecordingHotkey}
        isApplyingHotkey={hotkeySetup.isApplyingHotkey}
        configHotkey={configHook.config.hotkey}
        systemShortcutContext={hotkeySetup.systemShortcutContext}
        hotkeyBindingState={hotkeySetup.hotkeyBindingState}
        updateResult={updatesHook.updateResult}
        appVersion={appVersion}
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
        onCloseModelGuide={() => setShowModelGuide(false)}
      />
    </div>
  );
}

export default App;