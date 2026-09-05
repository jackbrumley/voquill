import {
  IconCheck,
  IconInfoCircle,
  IconRefresh,
  IconShieldLock,
} from '@tabler/icons-preact';
import { Button } from '../components/Button.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';
import { SelectField } from '../components/SelectField.tsx';
import { SettingRow } from '../components/SettingRow.tsx';
import { helperTextStyle, inputBaseStyle, selectWrapperStyle, tabPanelPaddedStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import { API_KEY_PLACEHOLDER, type ReadinessStatus } from '../readiness.ts';
import type { AudioDevice, DownloadPhase, GpuStatus } from '../types.ts';

interface ModelInfo {
  engine: string;
  size: string;
  file_size: number;
  download_url: string;
  sha256: string;
  label: string;
  description: string;
  recommended: boolean;
}

interface LinuxPermissions {
  audio: boolean;
  shortcuts: boolean;
  input_emulation: boolean;
  shortcuts_status: string;
  shortcuts_detail?: string;
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
  openai_api_key: string;
  post_process_enabled: boolean;
  post_process_provider: 'Local' | 'API';
  post_process_model: string;
}

interface InitialSetupPageProps {
  permissions: LinuxPermissions | null;
  config: SetupConfig;
  readiness: ReadinessStatus;
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  isDownloading: boolean;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  systemShortcutContext: SystemShortcutContext | null;
  isApplyingHotkey: boolean;
  hotkeyError: string | null;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  isMicTriggered?: boolean;
  micTestPassed: boolean;
  gpuStatus: GpuStatus | null;
  isTestingEngine: boolean;
  isTestingApi: boolean;
  isRecordingHotkey: boolean;
  setupTouched: boolean;
  onTouchSetup: () => void;
  onAudioSetup: () => void;
  onInputSetup: () => void;
  onConfigureHotkey: () => void;
  onHotkeyKeyDown: (event: KeyboardEvent) => void;
  onHotkeyKeyUp: (event: KeyboardEvent) => void;
  onHotkeyBlur: () => void;
  onChangeConfig: (key: string, value: string | number | boolean | null) => void;
  onSelectEngine: (engine: string) => void;
  onShowModelGuide: () => void;
  onDownloadModel: (size: string) => void;
  onDownloadPostProcessModel: (size: string) => void;
  onRetryModels: () => void;
  onLoadMics: () => void;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
  onRefreshStatus: () => void;
  onTestApiKey: () => void;
  onFinishSetup: () => void;
}

export function InitialSetupPage(props: InitialSetupPageProps) {
  const {
    permissions,
    config,
    readiness,
    availableEngines,
    availableModels,
    modelStatus,
    downloadProgress,
    downloadPhase,
    isDownloading,
    portalVersion,
    isSystemManagedShortcut,
    isApplyingHotkey,
    hotkeyError,
    availableMics,
    micTestStatus,
    micVolume,
    isMicTriggered,
    micTestPassed,
    gpuStatus,
    isTestingEngine,
    isTestingApi,
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
    onSelectEngine,
    onShowModelGuide,
    onDownloadModel,
    onDownloadPostProcessModel,
    onRetryModels,
    onLoadMics,
    onStartMicTest,
    onStopMicTest,
    onStopMicPlayback,
    onRefreshStatus,
    onTestApiKey,
    onFinishSetup,
  } = props;

  const setupGhostPillStyle = {
    borderRadius: '40px',
    padding: '10px 24px',
    fontWeight: 700,
  } as const;

  const warningTextStyle = { ...helperTextStyle, color: '#f1c40f' } as const;
  const isGpuEngineSelected = config.local_engine.includes('(GPU)');

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto' }} key="initial-setup">
      <div style={{ ...tabPanelPaddedStyle, flex: 1 }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: tokens.spacing.md, textAlign: 'center', width: '100%', paddingTop: tokens.spacing.md }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '12px', background: 'rgba(255,255,255,0.05)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: tokens.colors.accentPrimary, boxShadow: '0 2px 10px rgba(0,0,0,0.2)' }}>
            <IconShieldLock size={32} />
          </div>
          <h2 style={{ margin: 0, fontSize: '24px', fontWeight: 800, letterSpacing: '-0.02em', color: tokens.colors.textPrimary }}>Initial Setup</h2>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md, color: '#d9dfe7', lineHeight: 1.6, textAlign: 'left' }}>
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
              className={`permission-item ${permissions?.shortcuts && !hotkeyError ? 'ready' : ''}`}
              title="Global Shortcuts"
              description="Required for the hotkey"
              status={permissions?.shortcuts && !hotkeyError ? (
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
              {!permissions?.shortcuts && permissions?.shortcuts_detail && (
                <div style={warningTextStyle}>
                  {permissions.shortcuts_detail}
                </div>
              )}
              {hotkeyError && (
                <div style={warningTextStyle}>
                  {hotkeyError}
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
              className={`permission-item ${readiness.isTranscriptionReady ? 'ready' : ''}`}
              title="Transcription Backend"
              description={
                config.transcription_mode === 'Local'
                  ? `Model ${config.local_model_size} is required for local transcription.`
                  : 'A valid API key is required for cloud transcription.'
              }
              status={readiness.isTranscriptionReady ? <IconCheck color="var(--colors-success)" size={20} /> : null}
            >
              {config.transcription_mode === 'Local' ? (
                <>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
                    <SelectField
                      value={config.local_engine}
                      onChange={(engine) => {
                        onTouchSetup();
                        onSelectEngine(engine);
                      }}
                      searchable={false}
                      ariaLabel="Transcription engine"
                      options={availableEngines.map((engine) => ({
                        value: engine,
                        label: engine.includes('(GPU)') ? `${engine} — Recommended for dedicated GPUs` : engine,
                      }))}
                      style={{ minWidth: 0 }}
                    />
                    <div style={helperTextStyle}>
                      GPU engines transcribe on your graphics card — dramatically faster than CPU when one is available.
                    </div>
                  </div>

                  {isGpuEngineSelected && isTestingEngine && (
                    <div style={helperTextStyle}>Testing GPU acceleration...</div>
                  )}
                  {isGpuEngineSelected && !isTestingEngine && gpuStatus?.tested && gpuStatus.available && (
                    <div style={{ ...helperTextStyle, color: tokens.colors.success }}>
                      GPU acceleration active — transcription will run on your graphics card.
                    </div>
                  )}
                  {isGpuEngineSelected && !isTestingEngine && gpuStatus?.tested && !gpuStatus.available && (
                    <div style={warningTextStyle}>
                      GPU unavailable: {gpuStatus.detail || 'No compatible GPU detected.'} Voquill will fall back to the CPU engine automatically.
                    </div>
                  )}
                  {isGpuEngineSelected && !isTestingEngine && !gpuStatus?.tested && (
                    <div style={helperTextStyle}>
                      GPU acceleration is verified automatically once a model is downloaded.
                    </div>
                  )}

                  <ModelSelectionPanel
                    availableModels={availableModels}
                    localEngine={config.local_engine}
                    localModelSize={config.local_model_size}
                    modelStatus={modelStatus}
                    isDownloading={isDownloading}
                    downloadProgress={downloadProgress}
                    downloadPhase={downloadPhase}
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
                </>
              ) : (
                <div style={selectWrapperStyle}>
                  <input
                    style={inputBaseStyle}
                    type="text"
                    value={config.openai_api_key === API_KEY_PLACEHOLDER ? '' : config.openai_api_key}
                    onChange={(event: Event) => {
                      onTouchSetup();
                      onChangeConfig('openai_api_key', (event.target as HTMLInputElement).value);
                    }}
                    placeholder="sk-..."
                    aria-label="API key"
                  />
                  <Button variant="configAction" size="sm" onClick={onTestApiKey} disabled={isTestingApi}>
                    {isTestingApi ? '...' : 'Test'}
                  </Button>
                </div>
              )}
            </SettingRow>

            <SettingRow
              className={`permission-item ${readiness.isAudioDeviceReady ? 'ready' : ''}`}
              title="Audio Device"
              description="Select the microphone Voquill should use."
              status={readiness.isAudioDeviceReady ? (
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
              {readiness.audioDeviceIssue === 'no-devices' && (
                <div style={warningTextStyle}>
                  No microphones detected. Connect a microphone, then press Refresh.
                </div>
              )}
              {readiness.audioDeviceIssue === 'device-missing' && (
                <div style={warningTextStyle}>
                  The previously selected microphone is not connected. Choose another device below.
                </div>
              )}
              {readiness.audioDeviceIssue !== 'no-devices' && (
                <SelectField
                  value={config.audio_device || 'default'}
                  onChange={(nextMicId) => {
                    onTouchSetup();
                    onChangeConfig('audio_device', nextMicId);
                  }}
                  options={availableMics.map((mic) => ({ value: mic.id, label: mic.label || mic.id }))}
                  ariaLabel="Setup microphone"
                />
              )}
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
                isMicTriggered={isMicTriggered}
                onStartMicTest={onStartMicTest}
                onStopMicTest={onStopMicTest}
                onStopMicPlayback={onStopMicPlayback}
              />
            </SettingRow>

            {readiness.postProcessModelMissing && (
              <SettingRow
                className="permission-item"
                title="Post-Processing Model"
                description={`Post-processing is enabled but the ${config.post_process_model} model is not downloaded — transcript cleanup will fail until it is.`}
                status={<IconInfoCircle size={20} color="#f1c40f" />}
              >
                <div style={{ display: 'flex', justifyContent: 'flex-start', width: '100%' }}>
                  <Button
                    variant="configAction"
                    size="sm"
                    disabled={isDownloading}
                    onClick={() => {
                      onTouchSetup();
                      onDownloadPostProcessModel(config.post_process_model);
                    }}
                  >
                    {isDownloading ? '...' : 'Download'}
                  </Button>
                </div>
              </SettingRow>
            )}

          </div>

        </div>

        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginTop: '6px', paddingBottom: tokens.spacing.xl }}>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: tokens.spacing.sm }}>
            <Button variant="ghost" onClick={onRefreshStatus} title="Refresh Status" style={{ width: '38px', height: '38px', padding: 0, borderRadius: '999px', color: tokens.colors.textPrimary }}>
              <IconRefresh size={16} />
            </Button>
            <Button
              variant="configAction"
              disabled={!readiness.isAllReady}
              onClick={onFinishSetup}
              style={{ minWidth: '180px' }}
            >
              Finish Setup
            </Button>
          </div>

          {!readiness.isAllReady && setupTouched && (
            <div style={{ marginTop: '8px', fontSize: '11px', color: '#d9dfe7', textAlign: 'center' }}>
              Complete all required items to finish setup.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
