import { Button } from './Button.tsx';
import { ActionFooter } from './ActionFooter.tsx';
import { StatusPage } from '../pages/StatusPage.tsx';
import { ConfigPage } from '../pages/ConfigPage.tsx';
import { HistoryPage } from '../pages/HistoryPage.tsx';
import { UiLabPage } from '../pages/UiLabPage.tsx';
import { appContentStyle, tabNavStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import type { Config, HistoryItem, AudioDevice, EngineCapabilities, GpuStatus, HotkeyBindingState, OverlayPositioningCapabilities, ModelInfo, UpdateCheckResult, AppRoute } from '../types.ts';

interface MainLayoutProps {
  activeRoute: AppRoute;
  config: Config;
  currentStatus: string;
  appVersion: string;
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
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
  hoveredTopTab: AppRoute | null;
  history: HistoryItem[];
  updateResult: UpdateCheckResult | null;
  gpuStatus: GpuStatus | null;
  engineCapabilities: EngineCapabilities | null;
  onNavigate: (route: AppRoute) => void;
  onLogUI: (msg: string) => void;
  onSetHoveredTab: (route: AppRoute | null) => void;
  onSetActiveConfigSection: (value: string | null) => void;
  onUpdateConfig: (key: string, value: string | number | boolean | null | Record<string, unknown>) => void;
  onTestApiKey: () => void;
  onDownloadModel: (size: string) => void;
  onLoadModels: () => void;
  onLoadMics: () => void;
  onHandleConfigureHotkey: () => void;
  onSetShowModelGuide: (v: boolean) => void;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
  onOpenDebugFolder: () => void;
  onReopenInitialSetup: () => void;
  onFactoryReset: () => void;
  onCheckForUpdates: () => void;
  onOpenUiLab: () => void;
  onToggleAutostart: (enabled: boolean) => void;
  onCopyToClipboard: (text: string) => void;
  onClearHistory: () => void;
  onToggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
  onOpenUpdateModal: () => void;
  tabContentRef: { current: HTMLDivElement | null };
}

export function MainLayout(props: MainLayoutProps) {
  const topTabBaseStyle = {
    flex: 1,
    padding: '12px 8px',
    textAlign: 'center' as const,
    cursor: 'pointer',
    border: 'none',
    background: 'transparent',
    fontSize: tokens.typography.sizeSm,
    fontWeight: 500 as const,
    transition: 'color 0.15s, background 0.15s',
    position: 'relative' as const,
    zIndex: 1,
    marginBottom: 0,
    borderRadius: '8px 8px 0 0',
  } as const;

  const getTopTabStyle = (route: AppRoute) => {
    const isActive = props.activeRoute === route;
    const isHovered = props.hoveredTopTab === route;
    return {
      ...topTabBaseStyle,
      background: isActive
        ? tokens.colors.bgTertiary
        : isHovered
          ? 'rgba(255, 255, 255, 0.05)'
          : 'transparent',
      color: isActive ? tokens.colors.textPrimary : tokens.colors.textSecondary,
      backdropFilter: isActive ? 'blur(5px)' : undefined,
      WebkitBackdropFilter: isActive ? 'blur(5px)' : undefined,
      boxShadow: isActive ? `inset 0 -1px 0 ${tokens.colors.bgPrimary}` : 'none',
    } as const;
  };

  return (
    <>
      <div style={tabNavStyle}>
        <button
          type="button"
          style={getTopTabStyle('status')}
          onClick={() => { props.onLogUI('Tab: Status'); props.onNavigate('status'); }}
          onMouseEnter={() => props.onSetHoveredTab('status')}
          onMouseLeave={() => props.onSetHoveredTab(null)}
          aria-current={props.activeRoute === 'status' ? 'page' : undefined}
        >
          Status
        </button>
        <button
          type="button"
          style={getTopTabStyle('history')}
          onClick={() => { props.onLogUI('Tab: History'); props.onNavigate('history'); }}
          onMouseEnter={() => props.onSetHoveredTab('history')}
          onMouseLeave={() => props.onSetHoveredTab(null)}
          aria-current={props.activeRoute === 'history' ? 'page' : undefined}
        >
          History
        </button>
        <button
          type="button"
          style={getTopTabStyle('settings')}
          onClick={() => { props.onLogUI('Tab: Settings'); props.onNavigate('settings'); }}
          onMouseEnter={() => props.onSetHoveredTab('settings')}
          onMouseLeave={() => props.onSetHoveredTab(null)}
          aria-current={props.activeRoute === 'settings' ? 'page' : undefined}
        >
          Settings
        </button>
      </div>

      <div style={appContentStyle} ref={props.tabContentRef}>
        {props.activeRoute === 'status' && (
          <StatusPage
            currentStatus={props.currentStatus}
            appVersion={props.appVersion}
            modelStatus={props.modelStatus}
            config={props.config}
            isSystemManagedShortcut={props.isSystemManagedShortcut}
            onToggleOutputMethod={props.onToggleOutputMethod}
            hasUpdateAvailable={props.updateResult?.updateAvailable === true}
            onOpenUpdateModal={props.onOpenUpdateModal}
          />
        )}

        {props.activeRoute === 'settings' && (
          <ConfigPage
            config={props.config}
            gpuStatus={props.gpuStatus}
            engineCapabilities={props.engineCapabilities}
            activeConfigSection={props.activeConfigSection}
            setActiveConfigSection={props.onSetActiveConfigSection}
            availableEngines={props.availableEngines}
            availableModels={props.availableModels}
            modelStatus={props.modelStatus}
            downloadProgress={props.downloadProgress}
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
          />
        )}

        {props.activeRoute === 'history' && (
          <HistoryPage history={props.history} onCopyToClipboard={props.onCopyToClipboard} />
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