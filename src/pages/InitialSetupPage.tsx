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

  return (
    <div className="tab-panel page-scroll initial-setup-page" key="initial-setup">
      <SurfaceCard className="tab-panel-content initial-setup-content">
        <div className="setup-header">
          <div className="setup-icon-container">
            <IconShieldLock size={32} className="setup-icon" />
          </div>
          <h2>Initial Setup</h2>
        </div>

        <div className="setup-body">
          <p className="setup-intro">Complete these required checks before first use:</p>
          <div className="setup-list">
            <div className="setup-section-label">Required</div>

            <SettingRow
              className={`permission-item ${permissions?.audio ? 'ready' : ''}`}
              title="Audio Access"
              description="Required for dictation"
              status={permissions?.audio ? (
                <IconCheck color="var(--colors-success)" size={20} />
              ) : (
                <Button variant="ghost" size="sm" onClick={onAudioSetup}>Request</Button>
              )}
            />

            <SettingRow
              className={`permission-item ${permissions?.shortcuts ? 'ready' : ''}`}
              title="Global Shortcuts"
              description="Required for the hotkey"
              status={permissions?.shortcuts ? (
                <IconCheck color="var(--colors-success)" size={20} />
              ) : (
                <Button variant="ghost" size="sm" onClick={onConfigureHotkey} disabled={isApplyingHotkey}>
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
                  className="hotkey-input setup-hotkey-input setup-hotkey-input-field"
                  style={{
                    cursor: portalVersion >= 1 ? 'default' : 'pointer',
                    color: isRecordingHotkey ? 'var(--colors-text-primary)' : 'var(--colors-text-primary)',
                    opacity: portalVersion >= 1 ? 0.85 : 1,
                  }}
                  title={portalVersion >= 1 ? 'Use Configure Hotkey to request a system shortcut.' : ''}
                />
              )}
              {isSystemManagedShortcut && (
                <div className="permission-desc permission-desc-note">
                  {systemShortcutContext?.distro
                    ? `Your ${systemShortcutContext.distro} system manages this shortcut. Change it in ${systemShortcutContext.settings_path}.`
                    : `Your system manages this shortcut. Change it in ${systemShortcutContext?.settings_path || 'System Settings -> Keyboard Shortcuts'}.`}
                </div>
              )}
              {!permissions?.shortcuts && permissions?.shortcuts_detail && (
                <div className="permission-desc permission-desc-warning">
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
                <Button variant="ghost" size="sm" onClick={onInputSetup}>Request</Button>
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
                <Button variant="ghost" size="sm" onClick={() => {
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
              >
                <option value="default">Default microphone</option>
                {availableMics.map((mic) => (
                  <option key={mic.id} value={mic.id}>{mic.label || mic.id}</option>
                ))}
              </select>
            </SettingRow>

            <div className="setup-section-label setup-section-recommended">Recommended</div>

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

          <p className="setup-note">
            Complete required checks to unlock the app. Mic Test is optional but recommended.
            {isSystemManagedShortcut && (
              <> To change your shortcut later, use {systemShortcutContext?.settings_path || 'System Settings -> Keyboard Shortcuts'}.</>
            )}
          </p>
        </div>

        <div className="setup-actions setup-button-container">
          <div className="setup-actions-row">
            <Button variant="ghost" onClick={onRefreshStatus} className="setup-refresh-button" title="Refresh Status">
              <IconRefresh size={16} />
            </Button>
            <Button
              variant="configAction"
              className="setup-finish-button"
              disabled={!isAllReady}
              onClick={onFinishSetup}
            >
              Finish Setup
            </Button>
          </div>

          {!isAllReady && setupTouched && (
            <div className="setup-actions-note">
              Complete all required items to finish setup.
            </div>
          )}
        </div>
      </SurfaceCard>
    </div>
  );
}
