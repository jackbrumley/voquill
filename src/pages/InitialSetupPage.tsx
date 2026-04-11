import {
  IconCheck,
  IconInfoCircle,
  IconRefresh,
  IconShieldLock,
} from '@tabler/icons-preact';
import { Button } from '../components/Button.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';
import { SurfaceCard } from '../components/SurfaceCard.tsx';
import { SettingRow } from '../components/SettingRow.tsx';
import { helperTextStyle, inputBaseStyle, selectBaseStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

interface AudioDevice {
  id: string;
  label: string;
}

interface LinuxPermissions {
  audio: boolean;
  shortcuts: boolean;
  input_emulation: boolean;
  shortcuts_status: string;
  shortcuts_detail?: string;
}

interface PortalDiagnostics {
  available: boolean;
  version: number;
  supports_configure_shortcuts: boolean;
}

interface SystemShortcutContext {
  distro?: string;
  desktop?: string;
  settings_path: string;
}

interface SetupConfig {
  transcription_mode: 'API' | 'Local';
  local_model_size: string;
  local_engine: string;
  hotkey: string;
  audio_device: string | null;
  input_sensitivity: number;
}

interface InitialSetupPageProps {
  permissions: LinuxPermissions | null;
  config: SetupConfig;
  availableModels: any[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
  isDownloading: boolean;
  portalVersion: number;
  portalDiagnostics: PortalDiagnostics | null;
  isSystemManagedShortcut: boolean;
  systemShortcutContext: SystemShortcutContext | null;
  isApplyingHotkey: boolean;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  micTestPassed: boolean;
  isLocalModelReady: boolean;
  isAudioDeviceReady: boolean;
  isAllReady: boolean;
  isRecordingHotkey: boolean;
  setupTouched: boolean;
  onTouchSetup: () => void;
  onAudioSetup: () => void;
  onInputSetup: () => void;
  onConfigureHotkey: () => void;
  onHotkeyKeyDown: (event: KeyboardEvent) => void;
  onHotkeyKeyUp: (event: KeyboardEvent) => void;
  onHotkeyBlur: () => void;
  onChangeConfig: (key: string, value: any) => void;
  onShowModelGuide: () => void;
  onDownloadModel: (size: string) => void;
  onRetryModels: () => void;
  onLoadMics: () => void;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
  onRefreshStatus: () => void;
  onFinishSetup: () => void;
}

export function InitialSetupPage(props: InitialSetupPageProps) {
  const {
    permissions,
    config,
    availableModels,
    modelStatus,
    downloadProgress,
    isDownloading,
    portalVersion,
    isSystemManagedShortcut,
    systemShortcutContext,
    isApplyingHotkey,
    availableMics,
    micTestStatus,
    micVolume,
    micTestPassed,
    isLocalModelReady,
    isAudioDeviceReady,
    isAllReady,
    isRecordingHotkey,
    setupTouched,
    onTouchSetup,
    onAudioSetup,
    onInputSetup,
    onConfigureHotkey,
    onHotkeyKeyDown,
    onHotkeyKeyUp,
    onHotkeyBlur,
    onChangeConfig,
    onShowModelGuide,
    onDownloadModel,
    onRetryModels,
    onLoadMics,
    onStartMicTest,
    onStopMicTest,
    onStopMicPlayback,
    onRefreshStatus,
    onFinishSetup,
  } = props;

  const setupGhostPillStyle = {
    borderRadius: '40px',
    padding: '10px 24px',
    fontWeight: 700,
  } as const;

  return (
    <div style={{ ...tabPanelStyle, overflow: 'hidden', padding: 0, height: '100%', minHeight: 0 }} key="initial-setup">
      <SurfaceCard
        className="tab-panel-content"
        style={{
          flex: 1,
          minHeight: 0,
          display: 'flex',
          flexDirection: 'column',
          background: 'transparent',
          border: 'none',
          boxShadow: 'none',
          backdropFilter: 'none',
          WebkitBackdropFilter: 'none',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.lg, width: '100%', maxWidth: '900px', margin: '0 auto', borderRadius: tokens.radii.panel, padding: tokens.spacing.md, height: '100%', minHeight: 0 }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: tokens.spacing.md, textAlign: 'center', width: '100%' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '12px', background: 'rgba(255,255,255,0.05)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: tokens.colors.accentPrimary, boxShadow: '0 2px 10px rgba(0,0,0,0.2)' }}>
            <IconShieldLock size={32} />
          </div>
          <h2 style={{ margin: 0, fontSize: '24px', fontWeight: 800, letterSpacing: '-0.02em', color: tokens.colors.textPrimary }}>Initial Setup</h2>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md, color: '#d9dfe7', lineHeight: 1.6, textAlign: 'left', flex: 1, minHeight: 0, overflowY: 'auto', paddingRight: '4px' }}>
          <p style={{ margin: 0, fontSize: tokens.typography.sizeSm, width: '100%', textAlign: 'center' }}>Complete these required checks before first use:</p>
          <div>
            <div style={{ fontSize: '11px', fontWeight: 800, letterSpacing: '0.08em', textTransform: 'uppercase', color: '#d9dfe7', margin: '2px 0 6px' }}>Required</div>

            <SettingRow
              className={`permission-item ${permissions?.audio ? 'ready' : ''}`}
              title="Audio Access"
              description="Required for dictation"
              status={permissions?.audio ? (
                <IconCheck color="var(--colors-success)" size={20} />
              ) : (
                <Button variant="ghost" size="sm" pill style={setupGhostPillStyle} onClick={onAudioSetup}>Request</Button>
              )}
            />

            <SettingRow
              className={`permission-item ${permissions?.shortcuts ? 'ready' : ''}`}
              title="Global Shortcuts"
              description="Required for the hotkey"
              status={permissions?.shortcuts ? (
                <IconCheck color="var(--colors-success)" size={20} />
              ) : (
                <Button variant="ghost" size="sm" pill style={setupGhostPillStyle} onClick={onConfigureHotkey} disabled={isApplyingHotkey}>
                  Change Shortcut
                </Button>
              )}
            >
              {!permissions?.shortcuts && !isSystemManagedShortcut && (
                <input
                  type="text"
                  value={isRecordingHotkey ? 'Press keys...' : config.hotkey}
                  onKeyDown={onHotkeyKeyDown}
                  onKeyUp={onHotkeyKeyUp}
                  onFocus={() => null}
                  onBlur={onHotkeyBlur}
                  readOnly
                  placeholder={portalVersion >= 1 ? 'Bind with button' : 'Click to set'}
                  className="hotkey-input"
                  style={{
                    ...inputBaseStyle,
                    width: '100%',
                    maxWidth: '240px',
                    textAlign: 'left',
                    cursor: portalVersion >= 1 ? 'default' : 'pointer',
                    color: tokens.colors.textPrimary,
                    opacity: portalVersion >= 1 ? 0.85 : 1,
                  }}
                  title={portalVersion >= 1 ? 'Use Configure Hotkey to request a system shortcut.' : ''}
                />
              )}
              {isSystemManagedShortcut && (
                <div style={helperTextStyle}>
                  {systemShortcutContext?.distro
                    ? `Your ${systemShortcutContext.distro} system manages this shortcut. Change it in ${systemShortcutContext.settings_path}.`
                    : `Your system manages this shortcut. Change it in ${systemShortcutContext?.settings_path || 'System Settings -> Keyboard Shortcuts'}.`}
                </div>
              )}
              {!permissions?.shortcuts && permissions?.shortcuts_detail && (
                <div style={{ ...helperTextStyle, color: '#f1c40f' }}>
                  {permissions.shortcuts_detail}
                </div>
              )}
            </SettingRow>

            <SettingRow
              className={`permission-item ${permissions?.input_emulation ? 'ready' : ''}`}
              title="Input Simulation"
              description="Required to type into other apps"
              status={permissions?.input_emulation ? (
                <IconCheck color="var(--colors-success)" size={20} />
              ) : (
                <Button variant="ghost" size="sm" pill style={setupGhostPillStyle} onClick={onInputSetup}>Request</Button>
              )}
            />

            <SettingRow
              className={`permission-item ${isLocalModelReady ? 'ready' : ''}`}
              title="Transcription Backend"
              description={
                config.transcription_mode === 'Local'
                  ? `Model ${config.local_model_size} is required for local transcription.`
                  : 'API mode selected.'
              }
              status={isLocalModelReady ? <IconCheck color="var(--colors-success)" size={20} /> : null}
            >
              {config.transcription_mode === 'Local' && (
                <ModelSelectionPanel
                  availableModels={availableModels}
                  localEngine={config.local_engine}
                  localModelSize={config.local_model_size}
                  modelStatus={modelStatus}
                  isDownloading={isDownloading}
                  downloadProgress={downloadProgress}
                  actionButtonSize="sm"
                  onChangeModel={(size) => {
                    onTouchSetup();
                    onChangeConfig('local_model_size', size);
                  }}
                  onShowModelGuide={onShowModelGuide}
                  onDownloadModel={(size) => {
                    onTouchSetup();
                    onDownloadModel(size);
                  }}
                  onRetryModels={() => {
                    onTouchSetup();
                    onRetryModels();
                  }}
                />
              )}
            </SettingRow>

            <SettingRow
              className={`permission-item ${isAudioDeviceReady ? 'ready' : ''}`}
              title="Audio Device"
              description="Select the microphone Voquill should use."
              status={isAudioDeviceReady ? (
                <IconCheck color="var(--colors-success)" size={20} />
              ) : (
                <Button variant="ghost" size="sm" pill style={setupGhostPillStyle} onClick={() => {
                  onTouchSetup();
                  onLoadMics();
                }}>
                  <IconRefresh size={14} />
                  Refresh
                </Button>
              )}
            >
              <select
                value={config.audio_device || 'default'}
                onChange={(event) => {
                  onTouchSetup();
                  onChangeConfig('audio_device', (event.target as HTMLSelectElement).value);
                }}
                className="setup-audio-select"
                style={selectBaseStyle}
              >
                <option value="default">Default microphone</option>
                {availableMics.map((mic) => (
                  <option key={mic.id} value={mic.id}>{mic.label || mic.id}</option>
                ))}
              </select>
            </SettingRow>

            <div style={{ fontSize: '11px', fontWeight: 800, letterSpacing: '0.08em', textTransform: 'uppercase', color: '#d9dfe7', margin: '8px 0 6px' }}>Recommended</div>

            <SettingRow
              className={`permission-item ${micTestPassed ? 'ready' : ''}`}
              title="Mic Test (Recommended)"
              description="Record a short sample and play it back to verify your setup."
              status={<IconInfoCircle size={20} color="var(--colors-text-secondary)" />}
            >
              <MicSetupPanel
                compact
                inputSensitivity={config.input_sensitivity}
                actionButtonSize="sm"
                onInputSensitivityChange={(value) => onChangeConfig('input_sensitivity', value)}
                micTestStatus={micTestStatus}
                micVolume={micVolume}
                onStartMicTest={onStartMicTest}
                onStopMicTest={onStopMicTest}
                onStopMicPlayback={onStopMicPlayback}
              />
            </SettingRow>
          </div>

        </div>

        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginTop: '6px', paddingBottom: tokens.spacing.md }}>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: tokens.spacing.sm }}>
            <Button variant="ghost" onClick={onRefreshStatus} title="Refresh Status" style={{ width: '38px', height: '38px', padding: 0, borderRadius: '999px', color: tokens.colors.textPrimary }}>
              <IconRefresh size={16} />
            </Button>
            <Button
              variant="configAction"
              disabled={!isAllReady}
              onClick={onFinishSetup}
              style={{ minWidth: '180px' }}
            >
              Finish Setup
            </Button>
          </div>

          {!isAllReady && setupTouched && (
            <div style={{ marginTop: '8px', fontSize: '11px', color: '#d9dfe7', textAlign: 'center' }}>
              Complete all required items to finish setup.
            </div>
          )}
        </div>
        </div>
      </SurfaceCard>
    </div>
  );
}
