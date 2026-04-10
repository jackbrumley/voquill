import { IconInfoCircle, IconRefresh, IconRocket } from '@tabler/icons-preact';
import { ConfigField } from '../components/ConfigField.tsx';
import { Switch } from '../components/Switch.tsx';
import { CollapsibleSection } from '../components/CollapsibleSection.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { Button } from '../components/Button.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';

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
  portalDiagnostics: { active_trigger?: string } | null;
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
  } = props;

  return (
    <div className="tab-panel config-panel page-scroll" key="config">
      <div className="tab-panel-content">
        <CollapsibleSection title="Basic" isOpen={activeConfigSection === 'basic'} onToggle={() => setActiveConfigSection(activeConfigSection === 'basic' ? null : 'basic')}>
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

          <ConfigField label="Language" description="Hint the dialect or hard-set the output language.">
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

          {config.transcription_mode === 'API' ? (
            <>
              <ConfigField label="API Key" description="Used to authenticate with the transcription service (OpenAI).">
                <div className="input-with-button" style={{ display: 'flex', gap: '8px' }}>
                  <input type="text" value={config.openai_api_key} onChange={(e: Event) => updateConfig('openai_api_key', (e.target as HTMLInputElement).value)} placeholder="sk-..." />
                  <Button onClick={testApiKey} disabled={isTestingApi}>{isTestingApi ? '...' : 'Test'}</Button>
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
                <div className="select-wrapper">
                  {availableModels.length > 0 ? (
                    <>
                      <select value={config.local_model_size} onChange={(e: Event) => updateConfig('local_model_size', (e.target as HTMLSelectElement).value)}>
                        {availableModels
                          .filter((m) => m.engine === config.local_engine)
                          .map((m) => (
                            <option key={m.size} value={m.size}>
                              {m.label} {m.recommended ? '(Recommended)' : ''} ({Math.round(m.file_size / 1024 / 1024)}MB)
                            </option>
                          ))}
                      </select>
                      <Button variant="ghost" className="icon-button" onClick={() => setShowModelGuide(true)} title="Model Guide">
                        <IconInfoCircle size={20} />
                      </Button>
                      {!modelStatus[config.local_model_size] && (
                        <Button size="sm" onClick={() => downloadModel(config.local_model_size)} disabled={isDownloading}>
                          {isDownloading ? '...' : 'Download'}
                        </Button>
                      )}
                    </>
                  ) : (
                    <div style={{ display: 'flex', gap: '8px', alignItems: 'center', width: '100%' }}>
                      <div style={{ fontSize: '12px', color: 'var(--color-text-dim)', flex: 1 }}>Loading models...</div>
                      <Button size="sm" onClick={loadModels}>Retry</Button>
                    </div>
                  )}
                </div>
                {availableModels.length > 0 && (
                  <div className="mode-description" style={{ textAlign: 'left', marginTop: '4px' }}>
                    {availableModels.find((m) => m.size === config.local_model_size)?.description}
                  </div>
                )}
              </ConfigField>
              {isDownloading && (
                <div className="download-progress-container" style={{ marginTop: '-8px', marginBottom: '16px' }}>
                  <div className="volume-meter-container" style={{ height: '4px' }}>
                    <div className="volume-meter-bar" style={{ width: `${Math.min(downloadProgress, 100)}%`, background: 'var(--colors-accent-primary)' }}></div>
                  </div>
                  <div style={{ fontSize: '10px', color: 'var(--color-text-dim)', textAlign: 'right', marginTop: '2px' }}>Downloading model... {Math.round(downloadProgress)}%</div>
                </div>
              )}
            </>
          )}

          <ConfigField label="Global Hotkey" description="Hold these keys to record, release to transcribe.">
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
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
              <Button size="sm" variant="secondary" onClick={handleConfigureHotkey} disabled={isApplyingHotkey}>
                Configure Hotkey
              </Button>
            </div>
            {portalVersion >= 1 && (
              <div style={{ fontSize: '11px', color: 'var(--colors-text-muted)', marginTop: '4px' }}>
                Shortcut registration uses the Wayland GlobalShortcuts portal.
                {portalDiagnostics?.active_trigger ? ` Active shortcut: ${portalDiagnostics.active_trigger}.` : ''}
                {hotkeyBindingState?.bound ? ' Listener is active.' : ''}
              </div>
            )}
          </ConfigField>

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
        </CollapsibleSection>

        <CollapsibleSection title="Advanced" isOpen={activeConfigSection === 'advanced'} onToggle={() => setActiveConfigSection(activeConfigSection === 'advanced' ? null : 'advanced')}>
          <ConfigField label="Popup Position (px)" description="Vertical offset for the status overlay from the screen bottom.">
            <input type="number" value={config.pixels_from_bottom} onChange={(e: Event) => updateConfig('pixels_from_bottom', parseInt((e.target as HTMLInputElement).value))} />
          </ConfigField>

          <ConfigField label="Debug Mode" description="Master switch for advanced diagnostic settings.">
            <Switch checked={config.debug_mode} onChange={(checked) => updateConfig('debug_mode', checked)} label="Enable Debug Settings" />
          </ConfigField>

          <ConfigField label="Turbo Mode (GPU)" description="Uses your graphics card to speed up transcription. Recommended for 'Medium' models.">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <Switch checked={config.enable_gpu} onChange={(checked) => updateConfig('enable_gpu', checked)} label="Enabled" />
              <IconRocket size={20} style={{ color: config.enable_gpu ? '#f1c40f' : 'var(--colors-text-muted)', opacity: config.enable_gpu ? 1 : 0.5, transition: 'all 0.3s ease' }} />
            </div>
          </ConfigField>

          {config.debug_mode && (
            <ConfigField label="Recording Logs" description="Saves dictation recordings as WAV files to your app data folder to help analyze audio issues.">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <Switch checked={config.enable_recording_logs} onChange={(checked) => updateConfig('enable_recording_logs', checked)} label="Enable Recording Logs" />
                <Button size="sm" variant="ghost" onClick={openDebugFolder}>Open Folder</Button>
              </div>
            </ConfigField>
          )}
        </CollapsibleSection>
      </div>
    </div>
  );
}
