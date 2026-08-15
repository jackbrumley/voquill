import { Button } from './Button.tsx';
import { ActionFooter } from './ActionFooter.tsx';
import { HomePage } from '../pages/HomePage.tsx';
import { HelpPage } from '../pages/HelpPage.tsx';
import { ConfigPage } from '../pages/ConfigPage.tsx';
import { HistoryPage } from '../pages/HistoryPage.tsx';
import { UiLabPage } from '../pages/UiLabPage.tsx';
import { TabBar } from './TabBar.tsx';
import { appContentStyle } from '../theme/ui-primitives.ts';
import type { Config, DownloadPhase, HistoryItem, AudioDevice, EngineCapabilities, GpuStatus, HotkeyBindingState, OverlayPositioningCapabilities, ModelInfo, UpdateCheckResult, AppRoute } from '../types.ts';

interface MainLayoutProps {
  activeRoute: AppRoute;
  config: Config;
  appVersion: string;
  sessionStatus: string;
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  isDownloading: boolean;
  isTestingApi: boolean;
  activeConfigSection: string | null;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  hotkeyBindingState: HotkeyBindingState | null;
  isApplyingHotkey: boolean;
  availableMics: AudioDevice[];
  micTestStatus: 'playing' | 'idle' | 'recording' | 'processing';
  micVolume: number;
  overlayPositioningCapabilities: OverlayPositioningCapabilities;
  checkingUpdates: boolean;
  autostartEnabled: boolean;
  history: HistoryItem[];
  searchQuery: string;
  searchResults: HistoryItem[];
  updateResult: UpdateCheckResult | null;
  gpuStatus: GpuStatus | null;
  postProcessGpuStatus: GpuStatus | null;
  engineCapabilities: EngineCapabilities | null;
  onNavigate: (route: AppRoute) => void;
  onLogUI: (msg: string) => void;
  onSetActiveConfigSection: (value: string | null) => void;
  onUpdateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown>) => void;
  onTestApiKey: () => void;
  onDownloadModel: (size: string, engine?: string) => void;
  onLoadModels: () => void;
  onLoadMics: () => void;
  onHandleConfigureHotkey: () => void;
  onSetShowModelGuide: (v: boolean) => void;
  onSetShowPostProcessGuide: (v: boolean) => void;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
  onOpenDebugFolder: () => void;
  onReopenInitialSetup: () => void;
  onFactoryReset: () => void;
  onCheckForUpdates: () => void;
  onOpenUiLab: () => void;
  onToggleAutostart: (enabled: boolean) => void;
  onTestCleanupApi: () => void;
  onCopyToClipboard: (text: string) => void;
  onClearHistory: () => void;
  onSearchHistory: (query: string) => void;
  onToggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
  onOpenUpdateModal: () => void;
  tabContentRef: { current: HTMLDivElement | null };
}

export function MainLayout(props: MainLayoutProps) {
  return (
    <>
      <TabBar
        active={props.activeRoute}
        tabs={[
          { value: 'home', label: 'Home' },
          { value: 'history', label: 'History' },
          { value: 'settings', label: 'Settings' },
          { value: 'help', label: 'Help' },
        ]}
        onNavigate={props.onNavigate}
        onLogUI={props.onLogUI}
      />

      <div style={appContentStyle} ref={props.tabContentRef}>
        {props.activeRoute === 'home' && (
          <HomePage
            appVersion={props.appVersion}
            config={props.config}
            sessionStatus={props.sessionStatus}
            onToggleOutputMethod={props.onToggleOutputMethod}
            onToggleDiarization={(enabled) => props.onUpdateConfig('diarization_enabled_files', enabled)}
            hasUpdateAvailable={props.updateResult?.updateAvailable === true}
            onOpenUpdateModal={props.onOpenUpdateModal}
            onCopyToClipboard={props.onCopyToClipboard}
          />
        )}

        {props.activeRoute === 'help' && (
          <HelpPage
            config={props.config}
            modelStatus={props.modelStatus}
            isSystemManagedShortcut={props.isSystemManagedShortcut}
          />
        )}

        {props.activeRoute === 'settings' && (
          <ConfigPage
            config={props.config}
            gpuStatus={props.gpuStatus}
            postProcessGpuStatus={props.postProcessGpuStatus}
            engineCapabilities={props.engineCapabilities}
            activeConfigSection={props.activeConfigSection}
            setActiveConfigSection={props.onSetActiveConfigSection}
            availableEngines={props.availableEngines}
            availableModels={props.availableModels}
            modelStatus={props.modelStatus}
            downloadProgress={props.downloadProgress}
            downloadPhase={props.downloadPhase}
            isDownloading={props.isDownloading}
            isTestingApi={props.isTestingApi}
            portalVersion={props.portalVersion}
            isSystemManagedShortcut={props.isSystemManagedShortcut}
            hotkeyBindingState={props.hotkeyBindingState}
            isApplyingHotkey={props.isApplyingHotkey}
            availableMics={props.availableMics}
            micTestStatus={props.micTestStatus}
            micVolume={props.micVolume}
            overlayPositioningCapabilities={props.overlayPositioningCapabilities}
            updateConfig={props.onUpdateConfig}
            testApiKey={props.onTestApiKey}
            downloadModel={props.onDownloadModel}
            loadModels={props.onLoadModels}
            loadMics={props.onLoadMics}
            handleConfigureHotkey={props.onHandleConfigureHotkey}
            setShowModelGuide={props.onSetShowModelGuide}
            setShowPostProcessGuide={props.onSetShowPostProcessGuide}
            startMicTest={props.onStartMicTest}
            stopMicTest={props.onStopMicTest}
            stopMicPlayback={props.onStopMicPlayback}
            openDebugFolder={props.onOpenDebugFolder}
            onReopenInitialSetup={props.onReopenInitialSetup}
            onFactoryReset={props.onFactoryReset}
            checkingUpdates={props.checkingUpdates}
            onCheckForUpdates={props.onCheckForUpdates}
            onOpenUiLab={props.onOpenUiLab}
            autostartEnabled={props.autostartEnabled}
            onToggleAutostart={props.onToggleAutostart}
            testCleanupApi={props.onTestCleanupApi}
          />
        )}

        {props.activeRoute === 'history' && (
          <HistoryPage
            history={props.history}
            searchQuery={props.searchQuery}
            searchResults={props.searchResults}
            onCopyToClipboard={props.onCopyToClipboard}
            onSearch={props.onSearchHistory}
          />
        )}

        {props.activeRoute === 'ui-lab' && (
          <UiLabPage
            appVersion={props.appVersion}
            onBackToSettings={() => props.onNavigate('settings')}
            onOpenUpdateModal={props.onOpenUpdateModal}
          />
        )}
      </div>

      {props.activeRoute === 'history' && (
        <ActionFooter>
          <Button variant="danger" pill floating onClick={props.onClearHistory}>Clear History</Button>
        </ActionFooter>
      )}
    </>
  );
}