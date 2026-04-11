import { IconRefresh, IconRocket } from '@tabler/icons-preact';
import { ConfigField } from '../components/ConfigField.tsx';
import { Switch } from '../components/Switch.tsx';
import { CollapsibleSection } from '../components/CollapsibleSection.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { Button } from '../components/Button.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';

interface AudioDevice {
  id: string;
  label: string;
}

interface SystemShortcutContext {
  distro?: string;
  desktop?: string;
  settings_path: string;
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
  portalDiagnostics: { active_trigger?: string } | null;
  isSystemManagedShortcut: boolean;
  systemShortcutContext: SystemShortcutContext | null;
  hotkeyBindingState: { bound: boolean } | null;
  isApplyingHotkey: boolean;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
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
}

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
    portalDiagnostics,
    isSystemManagedShortcut,
    systemShortcutContext,
    hotkeyBindingState,
    isApplyingHotkey,
    availableMics,
    micTestStatus,
    micVolume,
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
  } = props;

  return (
    <div className="tab-panel config-panel page-scroll" key="config">
      <div className="tab-panel-content">
        <CollapsibleSection title="Transcription" isOpen={activeConfigSection === 'transcription'} onToggle={() => setActiveConfigSection(activeConfigSection === 'transcription' ? null : 'transcription')}>
          <ConfigField
            label="Global Hotkey"
            description={isSystemManagedShortcut ? 'Use your system shortcut to record and release to transcribe.' : 'Hold these keys to record, release to transcribe.'}
          >
            <div className={`config-hotkey-row ${isSystemManagedShortcut ? 'is-system-managed' : ''}`}>
              {!isSystemManagedShortcut && (
                <input
                  type="text"
                  value={config.hotkey}
                  readOnly
                  onClick={() => {}}
                  placeholder="Configure using button"
                  className="hotkey-input"
                  style={{ opacity: portalVersion >= 1 ? 0.9 : 1, cursor: 'default' }}
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
            {isSystemManagedShortcut ? (
              <div className="config-inline-note">
                {systemShortcutContext?.distro
                  ? `Your ${systemShortcutContext.distro} system manages this shortcut. To change it, open ${systemShortcutContext.settings_path}.`
                  : `Your system manages this shortcut. To change it, open ${systemShortcutContext?.settings_path || 'System Settings -> Keyboard Shortcuts'}.`}
                {portalDiagnostics?.active_trigger ? ` Current shortcut: ${portalDiagnostics.active_trigger}.` : ''}
              </div>
            ) : portalVersion >= 1 && (
              <div className="config-inline-note">
                Shortcut registration uses the Wayland GlobalShortcuts portal.
                {portalDiagnostics?.active_trigger ? ` Active shortcut: ${portalDiagnostics.active_trigger}.` : ''}
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
                <div className="config-inline-actions input-with-button">
                  <input type="text" value={config.openai_api_key} onChange={(e: Event) => updateConfig('openai_api_key', (e.target as HTMLInputElement).value)} placeholder="sk-..." />
                  <Button variant="configAction" onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? '...' : 'Test'}</Button>
                </div>
              </ConfigField>

              <ConfigField label="API URL" description="The endpoint that processes audio (OpenAI or Local Whisper).">
                <input type="url" value={config.api_url} onChange={(e: Event) => updateConfig('api_url', (e.target as HTMLInputElement).value)} />
              </ConfigField>

              <ConfigField label="API Model" description="The model name to use with the API provider.">
                <input type="text" value={config.api_model} onChange={(e: Event) => updateConfig('api_model', (e.target as HTMLInputElement).value)} />
              </ConfigField>
            </>
          ) : (
            <>
              <ConfigField label="Local Engine" description="The core technology used to process your voice locally.">
                <div className="select-wrapper">
                  <select value={config.local_engine} onChange={(e: Event) => updateConfig('local_engine', (e.target as HTMLSelectElement).value)}>
                    {availableEngines.map((engine) => (
                      <option key={engine} value={engine}>{engine}</option>
                    ))}
                  </select>
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

              <ConfigField label="Turbo Mode (GPU)" description="Uses your graphics card to speed up transcription. Recommended for 'Medium' models.">
                <div className="config-inline-actions config-inline-actions-between">
                  <Switch checked={config.enable_gpu} onChange={(checked) => updateConfig('enable_gpu', checked)} label="Enabled" />
                  <IconRocket size={20} className={`turbo-icon-indicator ${config.enable_gpu ? 'is-enabled' : ''}`} />
                </div>
              </ConfigField>
            </>
          )}

          <ConfigField label="Always Copy to Clipboard" description="Automatically copies the transcription to your clipboard even when in Typewriter mode.">
            <Switch checked={config.copy_on_typewriter} onChange={(checked) => updateConfig('copy_on_typewriter', checked)} label="Enabled" />
          </ConfigField>

        </CollapsibleSection>

        <CollapsibleSection title="Audio" isOpen={activeConfigSection === 'audio'} onToggle={() => setActiveConfigSection(activeConfigSection === 'audio' ? null : 'audio')}>
          <ConfigField label="Microphone" description="Choose the input device for recording your voice.">
            <div className="select-wrapper">
              <select value={config.audio_device || 'default'} onChange={(e: Event) => updateConfig('audio_device', (e.target as HTMLSelectElement).value)}>
                {availableMics.map((mic) => <option key={mic.id} value={mic.id}>{mic.label}</option>)}
              </select>
              <Button variant="ghost" className="icon-button" onClick={loadMics} title="Refresh Devices">
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
            <input type="number" value={config.typing_speed_interval} onChange={(e: Event) => updateConfig('typing_speed_interval', parseInt((e.target as HTMLInputElement).value))} />
          </ConfigField>

          <ConfigField label="Key Press Duration (ms)" description="How long each key is held. Increase if characters are skipped.">
            <input type="number" value={config.key_press_duration_ms} onChange={(e: Event) => updateConfig('key_press_duration_ms', parseInt((e.target as HTMLInputElement).value))} />
          </ConfigField>

          <ConfigField label="Status Overlay Position (px)" description="Vertical offset for the status overlay from the bottom of the screen.">
            <input type="number" value={config.pixels_from_bottom} onChange={(e: Event) => updateConfig('pixels_from_bottom', parseInt((e.target as HTMLInputElement).value))} />
          </ConfigField>
        </CollapsibleSection>

        <CollapsibleSection title="Debug" isOpen={activeConfigSection === 'debug'} onToggle={() => setActiveConfigSection(activeConfigSection === 'debug' ? null : 'debug')}>
          <ConfigField label="Logs" description="Copy or open logs for troubleshooting and support.">
            <div className="config-inline-actions">
              <Button variant="configAction" onClick={onCopySessionLogs}>Copy Logs</Button>
              <Button variant="ghost" onClick={openSessionLog}>Open Log File</Button>
            </div>
          </ConfigField>

          <ConfigField label="Recording Logs" description="Saves dictation recordings as WAV files to your app data folder to help analyze audio issues.">
            <div className="config-inline-actions config-inline-actions-between">
              <Switch checked={config.enable_recording_logs} onChange={(checked) => updateConfig('enable_recording_logs', checked)} label="Enable Recording Logs" />
              <Button variant="ghost" onClick={openDebugFolder}>Open Folder</Button>
            </div>
          </ConfigField>

          <ConfigField label="Initial Setup" description="Re-open onboarding checks for permissions, model, and hotkey setup.">
            <Button variant="configAction" onClick={onReopenInitialSetup}>Re-run Initial Setup</Button>
          </ConfigField>

          <div className="config-subsection-divider" role="separator" aria-hidden="true"></div>
          <div className="config-subsection-heading">Experimental</div>

          <ConfigField label="Language Hint" description="Best-effort language hint for transcription. Some engines/models may ignore this setting or apply it inconsistently.">
            <div className="select-wrapper">
              <select value={config.language} onChange={(e: Event) => updateConfig('language', (e.target as HTMLSelectElement).value)}>
                <option value="auto">Automatic Detection</option>
                <option value="en-AU">English (Australia)</option>
                <option value="en-GB">English (United Kingdom)</option>
                <option value="en-US">English (United States)</option>
                <option value="fr">French</option>
                <option value="es">Spanish</option>
                <option value="de">German</option>
                <option value="it">Italian</option>
                <option value="pt">Portuguese</option>
                <option value="nl">Dutch</option>
                <option value="ja">Japanese</option>
                <option value="zh">Chinese</option>
              </select>
            </div>
          </ConfigField>
        </CollapsibleSection>
      </div>
    </div>
  );
}
