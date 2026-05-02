import { IconRefresh, IconRocket } from '@tabler/icons-preact';
import { ConfigField } from '../components/ConfigField.tsx';
import { Switch } from '../components/Switch.tsx';
import { CollapsibleSection } from '../components/CollapsibleSection.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { Button } from '../components/Button.tsx';
import { NumberField } from '../components/NumberField.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';
import { SelectField } from '../components/SelectField.tsx';
import { helperTextStyle, inputBaseStyle, selectWrapperStyle, tabPanelContentStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

interface AudioDevice {
  id: string;
  label: string;
}

interface ConfigPageProps {
  config: {
    transcription_mode: 'API' | 'Local';
    local_model_size: string;
    local_engine: string;
    hotkey: string;
    language: string;
    openai_api_key: string;
    api_url: string;
    api_model: string;
    copy_on_typewriter: boolean;
    audio_device: string | null;
    input_sensitivity: number;
    typing_speed_interval: number;
    key_press_duration_ms: number;
    pixels_from_bottom: number;
    debug_mode: boolean;
    enable_gpu: boolean;
    enable_recording_logs: boolean;
  };
  activeConfigSection: string | null;
  setActiveConfigSection: (value: string | null) => void;
  availableEngines: string[];
  availableModels: any[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
  isDownloading: boolean;
  isTestingApi: boolean;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  hotkeyBindingState: { bound: boolean; active_trigger?: string } | null;
  isApplyingHotkey: boolean;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  overlayPositioningCapabilities: { manual_offset_supported: boolean; detail?: string };
  updateConfig: (key: string, value: any) => void;
  testApiKey: () => void;
  downloadModel: (size: string) => void;
  loadModels: () => void;
  loadMics: () => void;
  handleConfigureHotkey: () => void;
  setShowModelGuide: (show: boolean) => void;
  startMicTest: () => void;
  stopMicTest: () => void;
  stopMicPlayback: () => void;
  openDebugFolder: () => void;
  openSessionLog: () => void;
  onReopenInitialSetup: () => void;
  onCopySessionLogs: () => void;
  onFactoryReset: () => void;
  checkingUpdates: boolean;
  onCheckForUpdates: () => void;
  onOpenUiLab: () => void;
}

const languageOptions = [
  { value: 'auto', label: 'Automatic Detection' },
  { value: 'en-AU', label: 'English (Australia)' },
  { value: 'en-GB', label: 'English (United Kingdom)' },
  { value: 'en-US', label: 'English (United States)' },
  { value: 'fr', label: 'French' },
  { value: 'es', label: 'Spanish' },
  { value: 'de', label: 'German' },
  { value: 'it', label: 'Italian' },
  { value: 'pt', label: 'Portuguese' },
  { value: 'nl', label: 'Dutch' },
  { value: 'ja', label: 'Japanese' },
  { value: 'zh', label: 'Chinese' },
];

export function ConfigPage(props: ConfigPageProps) {
  const {
    config,
    activeConfigSection,
    setActiveConfigSection,
    availableEngines,
    availableModels,
    modelStatus,
    downloadProgress,
    isDownloading,
    isTestingApi,
    portalVersion,
    isSystemManagedShortcut,
    hotkeyBindingState,
    isApplyingHotkey,
    availableMics,
    micTestStatus,
    micVolume,
    overlayPositioningCapabilities,
    updateConfig,
    testApiKey,
    downloadModel,
    loadModels,
    loadMics,
    handleConfigureHotkey,
    setShowModelGuide,
    startMicTest,
    stopMicTest,
    stopMicPlayback,
    openDebugFolder,
    openSessionLog,
    onReopenInitialSetup,
    onCopySessionLogs,
    onFactoryReset,
    checkingUpdates,
    onCheckForUpdates,
    onOpenUiLab,
  } = props;

  const configGhostPillStyle = {
    borderRadius: '40px',
    padding: '10px 24px',
    fontWeight: 700,
  } as const;

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto', padding: 0 }} key="config">
      <div style={{ ...tabPanelContentStyle, maxWidth: '100%', margin: 0 }}>
        <CollapsibleSection title="Transcription" isOpen={activeConfigSection === 'transcription'} onToggle={() => setActiveConfigSection(activeConfigSection === 'transcription' ? null : 'transcription')}>
          <ConfigField
            label="Global Hotkey"
            description={isSystemManagedShortcut ? 'Use your system shortcut to record and release to transcribe.' : 'Hold these keys to record, release to transcribe.'}
          >
            <div style={{ display: 'flex', gap: tokens.spacing.sm, alignItems: 'center', justifyContent: 'center', width: '100%' }}>
              {!isSystemManagedShortcut && (
                <input
                  type="text"
                  value={config.hotkey}
                  readOnly
                  onClick={() => {}}
                  placeholder="Configure using button"
                  style={{ ...inputBaseStyle, opacity: portalVersion >= 1 ? 0.9 : 1, cursor: 'default' }}
                  title={portalVersion >= 1 ? 'Use Configure Hotkey to request binding through the system portal.' : ''}
                />
              )}
              <Button
                size="md"
                variant="configAction"
                onClick={handleConfigureHotkey}
                disabled={isApplyingHotkey}
              >
                Modify
              </Button>
            </div>
            {!isSystemManagedShortcut && portalVersion >= 1 && (
              <div style={helperTextStyle}>
                Shortcut registration uses the Wayland GlobalShortcuts portal.
                {hotkeyBindingState?.active_trigger ? ` Active shortcut: ${hotkeyBindingState.active_trigger}.` : ''}
                {hotkeyBindingState?.bound ? ' Listener is active.' : ''}
              </div>
            )}
          </ConfigField>

          <ConfigField label="Transcription Method" description="Choose between cloud-based API or fully local processing.">
            <ModeSwitcher
              value={config.transcription_mode}
              onToggle={(val) => updateConfig('transcription_mode', val)}
              options={[
                { value: 'Local', label: 'Local', title: 'Run Whisper locally' },
                { value: 'API', label: 'Cloud API', title: 'Use OpenAI API' },
              ]}
            />
          </ConfigField>

          {config.transcription_mode === 'API' ? (
            <>
              <ConfigField label="API Key" description="Used to authenticate with the transcription service (OpenAI).">
                <div style={{ ...selectWrapperStyle }}>
                  <input style={inputBaseStyle} type="text" value={config.openai_api_key} onChange={(e: Event) => updateConfig('openai_api_key', (e.target as HTMLInputElement).value)} placeholder="sk-..." />
                  <Button variant="configAction" onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? '...' : 'Test'}</Button>
                </div>
              </ConfigField>

              <ConfigField label="API URL" description="The endpoint that processes audio (OpenAI or Local Whisper).">
                <input style={inputBaseStyle} type="url" value={config.api_url} onChange={(e: Event) => updateConfig('api_url', (e.target as HTMLInputElement).value)} />
              </ConfigField>

              <ConfigField label="API Model" description="The model name to use with the API provider.">
                <input style={inputBaseStyle} type="text" value={config.api_model} onChange={(e: Event) => updateConfig('api_model', (e.target as HTMLInputElement).value)} />
              </ConfigField>
            </>
          ) : (
            <>
              <ConfigField label="Local Engine" description="The core technology used to process your voice locally.">
                <div style={selectWrapperStyle}>
                  <SelectField
                    value={config.local_engine}
                    options={availableEngines.map((engine) => ({ value: engine, label: engine }))}
                    onChange={(nextEngine) => updateConfig('local_engine', nextEngine)}
                    ariaLabel="Local engine"
                  />
                </div>
              </ConfigField>

              <ConfigField label="Local Model" description="Choose the Whisper model size. Distil-Small is recommended for most users.">
                <ModelSelectionPanel
                  availableModels={availableModels}
                  localEngine={config.local_engine}
                  localModelSize={config.local_model_size}
                  modelStatus={modelStatus}
                  isDownloading={isDownloading}
                  downloadProgress={downloadProgress}
                  onChangeModel={(size) => updateConfig('local_model_size', size)}
                  onShowModelGuide={() => setShowModelGuide(true)}
                  onDownloadModel={downloadModel}
                  onRetryModels={loadModels}
                />
              </ConfigField>

            </>
          )}

          <ConfigField label="Always Copy to Clipboard" description="Automatically copies the transcription to your clipboard even when in Typewriter mode.">
            <Switch checked={config.copy_on_typewriter} onChange={(checked) => updateConfig('copy_on_typewriter', checked)} />
          </ConfigField>

        </CollapsibleSection>

        <CollapsibleSection title="Audio" isOpen={activeConfigSection === 'audio'} onToggle={() => setActiveConfigSection(activeConfigSection === 'audio' ? null : 'audio')}>
          <ConfigField label="Microphone" description="Choose the input device for recording your voice.">
            <div style={selectWrapperStyle}>
              <SelectField
                value={config.audio_device || 'default'}
                options={availableMics.map((mic) => ({ value: mic.id, label: mic.label }))}
                onChange={(nextMicId) => updateConfig('audio_device', nextMicId)}
                ariaLabel="Microphone"
              />
              <Button variant="icon" onClick={loadMics} title="Refresh Devices">
                <IconRefresh size={16} />
              </Button>
            </div>
          </ConfigField>

          <ConfigField label="Mic Test & Sensitivity" description="Adjust capture gain and verify your microphone playback.">
            <MicSetupPanel
              inputSensitivity={config.input_sensitivity}
              onInputSensitivityChange={(value) => updateConfig('input_sensitivity', value)}
              micTestStatus={micTestStatus}
              micVolume={micVolume}
              onStartMicTest={startMicTest}
              onStopMicTest={stopMicTest}
              onStopMicPlayback={stopMicPlayback}
            />
          </ConfigField>
        </CollapsibleSection>

        <CollapsibleSection title="Typing" isOpen={activeConfigSection === 'typing'} onToggle={() => setActiveConfigSection(activeConfigSection === 'typing' ? null : 'typing')}>
          <ConfigField label="Typing Speed (ms)" description="Delay between characters. Lower values are faster (1ms recommended).">
            <NumberField value={config.typing_speed_interval} onChange={(value) => updateConfig('typing_speed_interval', value)} min={1} />
          </ConfigField>

          <ConfigField label="Key Press Duration (ms)" description="How long each key is held. Increase if characters are skipped.">
            <NumberField value={config.key_press_duration_ms} onChange={(value) => updateConfig('key_press_duration_ms', value)} min={1} />
          </ConfigField>

          <ConfigField label="Status Overlay Position (px)" description="Vertical offset for the status overlay from the bottom of the screen.">
            <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
              <NumberField
                value={config.pixels_from_bottom}
                onChange={(value) => updateConfig('pixels_from_bottom', value)}
                min={0}
                disabled={!overlayPositioningCapabilities.manual_offset_supported}
              />
              {!overlayPositioningCapabilities.manual_offset_supported && (
                <div style={helperTextStyle}>
                  {overlayPositioningCapabilities.detail || 'Manual overlay position adjustment is not available on your system.'}
                </div>
              )}
            </div>
          </ConfigField>
        </CollapsibleSection>

        <CollapsibleSection title="Debug" isOpen={activeConfigSection === 'debug'} onToggle={() => setActiveConfigSection(activeConfigSection === 'debug' ? null : 'debug')}>
          <ConfigField label="Logs" description="Copy or open logs for troubleshooting and support.">
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: tokens.spacing.sm, flexWrap: 'wrap', width: '100%' }}>
              <Button variant="configAction" onClick={onCopySessionLogs}>Copy Logs</Button>
              <Button variant="ghost" pill style={configGhostPillStyle} onClick={openSessionLog}>Open Log File</Button>
            </div>
          </ConfigField>

          <ConfigField label="Updates" description="Check for newer Voquill releases.">
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: tokens.spacing.sm, flexWrap: 'wrap', width: '100%' }}>
              <Button variant="ghost" pill style={configGhostPillStyle} onClick={onCheckForUpdates} disabled={checkingUpdates}>
                {checkingUpdates ? 'Checking...' : 'Check for Updates'}
              </Button>
            </div>
          </ConfigField>

          <ConfigField label="Recording Logs" description="Saves dictation recordings as WAV files to your app data folder to help analyze audio issues.">
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'stretch', gap: tokens.spacing.sm, width: '100%' }}>
              <Switch checked={config.enable_recording_logs} onChange={(checked) => updateConfig('enable_recording_logs', checked)} />
              <div style={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
                <Button variant="ghost" pill style={configGhostPillStyle} onClick={openDebugFolder}>Open Folder</Button>
              </div>
            </div>
          </ConfigField>

          <ConfigField label="Initial Setup" description="Re-open onboarding checks for permissions, model, and hotkey setup.">
            <Button variant="configAction" onClick={onReopenInitialSetup}>Re-run Initial Setup</Button>
          </ConfigField>

          <ConfigField label="Factory Reset" description="Reset Voquill to defaults and clear models, logs, and history.">
            <Button variant="danger" pill onClick={onFactoryReset}>Reset App to Defaults</Button>
          </ConfigField>

          <div style={{ width: '100%', height: '1px', margin: `${tokens.spacing.sm} 0 0`, background: 'rgba(255, 255, 255, 0.1)' }} role="separator" aria-hidden="true"></div>
          <div style={{ width: '100%', marginTop: tokens.spacing.sm, fontSize: '11px', fontWeight: 800, letterSpacing: '0.08em', textTransform: 'uppercase', color: '#d9dfe7' }}>Experimental</div>

          <ConfigField label="UI Lab" description="Open the internal visual QA page for component and state previews.">
            <Button variant="ghost" pill style={configGhostPillStyle} onClick={onOpenUiLab}>Open UI Lab</Button>
          </ConfigField>

          {config.transcription_mode === 'Local' && (
            <ConfigField
              label="Turbo Mode (GPU)"
              description="Experimental. GPU acceleration can be faster on some systems, but performance varies by hardware and model."
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: tokens.spacing.sm, width: '100%' }}>
                <IconRocket size={20} color={config.enable_gpu ? '#f1c40f' : tokens.colors.textMuted} />
                <Switch checked={config.enable_gpu} onChange={(checked) => updateConfig('enable_gpu', checked)} />
              </div>
            </ConfigField>
          )}

          <ConfigField label="Language Hint" description="Best-effort language hint for transcription. Some engines/models may ignore this setting or apply it inconsistently.">
            <div style={selectWrapperStyle}>
              <SelectField
                value={config.language}
                options={languageOptions}
                onChange={(nextLanguage) => updateConfig('language', nextLanguage)}
                ariaLabel="Language hint"
              />
            </div>
          </ConfigField>
        </CollapsibleSection>
      </div>
    </div>
  );
}
