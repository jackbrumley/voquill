import {
  IconCheck,
  IconInfoCircle,
  IconKeyboard,
  IconMicrophone,
  IconRefresh,
  IconRocket,
  IconShieldLock,
  IconTextRecognition,
} from '@tabler/icons-preact';
import { Button } from '../components/Button.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';

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
    <div className="initial-setup-page">
      <div className="initial-setup-content">
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

            <div className={`permission-item ${permissions?.audio ? 'ready' : ''}`}>
              <div className="permission-main">
                <div className="permission-icon">
                  <IconMicrophone size={20} />
                </div>
                <div className="permission-info">
                  <div className="permission-title">Audio Access</div>
                  <div className="permission-desc">Required for dictation</div>
                </div>
              </div>
              <div className="permission-status">
                {permissions?.audio ? (
                  <IconCheck color="var(--colors-success)" size={20} />
                ) : (
                  <Button variant="ghost" size="sm" onClick={onAudioSetup}>Request</Button>
                )}
              </div>
            </div>

            <div className={`permission-item ${permissions?.shortcuts ? 'ready' : ''}`}>
              <div className="permission-main">
                <div className="permission-icon">
                  <IconKeyboard size={20} />
                </div>
                <div className="permission-info permission-info-wide permission-info-padded">
                  <div className="permission-title permission-title-with-control">
                    Global Shortcuts
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
                          color: isRecordingHotkey ? 'var(--colors-primary)' : 'var(--colors-text)',
                          opacity: portalVersion >= 1 ? 0.8 : 1,
                        }}
                        title={portalVersion >= 1 ? 'Use Configure Hotkey to request a system shortcut.' : ''}
                      />
                    )}
                  </div>
                  <div className="permission-desc">Required for the hotkey</div>
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
                </div>
              </div>
              <div className="permission-status">
                {permissions?.shortcuts ? (
                  <IconCheck color="var(--colors-success)" size={20} />
                ) : (
                  <Button variant="ghost" size="sm" onClick={onConfigureHotkey} disabled={isApplyingHotkey}>
                    Change Shortcut
                  </Button>
                )}
              </div>
            </div>

            <div className={`permission-item ${permissions?.input_emulation ? 'ready' : ''}`}>
              <div className="permission-main">
                <div className="permission-icon">
                  <IconTextRecognition size={20} />
                </div>
                <div className="permission-info">
                  <div className="permission-title">Input Simulation</div>
                  <div className="permission-desc">Required to type into other apps</div>
                </div>
              </div>
              <div className="permission-status">
                {permissions?.input_emulation ? (
                  <IconCheck color="var(--colors-success)" size={20} />
                ) : (
                  <Button variant="ghost" size="sm" onClick={onInputSetup}>Request</Button>
                )}
              </div>
            </div>

            <div className={`permission-item ${isLocalModelReady ? 'ready' : ''}`}>
              <div className="permission-main permission-main-top">
                <div className="permission-icon">
                  <IconRocket size={20} />
                </div>
                <div className="permission-info permission-info-wide">
                  <div className="permission-title">Transcription Backend</div>
                  <div className="permission-desc">
                    {config.transcription_mode === 'Local'
                      ? `Model ${config.local_model_size} is required for local transcription.`
                      : 'API mode selected.'}
                  </div>
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
                </div>
              </div>
              <div className="permission-status">
                {isLocalModelReady && <IconCheck color="var(--colors-success)" size={20} />}
              </div>
            </div>

            <div className={`permission-item ${isAudioDeviceReady ? 'ready' : ''}`}>
              <div className="permission-main">
                <div className="permission-icon">
                  <IconMicrophone size={20} />
                </div>
                <div className="permission-info permission-info-wide permission-info-padded">
                  <div className="permission-title">Audio Device</div>
                  <div className="permission-desc">Select the microphone Voquill should use.</div>
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
                </div>
              </div>
              <div className="permission-status">
                {isAudioDeviceReady ? (
                  <IconCheck color="var(--colors-success)" size={20} />
                ) : (
                  <Button variant="ghost" size="sm" onClick={() => {
                    onTouchSetup();
                    onLoadMics();
                  }}>
                    Refresh
                  </Button>
                )}
              </div>
            </div>

            <div className="setup-section-label setup-section-recommended">Recommended</div>

            <div className={`permission-item ${micTestPassed ? 'ready' : ''}`}>
              <div className="permission-main permission-main-top">
                <div className="permission-icon">
                  <IconInfoCircle size={20} />
                </div>
                <div className="permission-info permission-info-wide">
                  <div className="permission-title">Mic Test (Recommended)</div>
                  <div className="permission-desc">Record a short sample and play it back to verify your setup.</div>
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
                </div>
              </div>
            </div>
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
      </div>
    </div>
  );
}
