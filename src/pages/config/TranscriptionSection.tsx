import { IconInfoCircle } from '@tabler/icons-preact';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { ModeSwitcher } from '../../components/ModeSwitcher.tsx';
import { Button } from '../../components/Button.tsx';
import { NumberField } from '../../components/NumberField.tsx';
import { SliderField } from '../../components/SliderField.tsx';
import { ModelSelectionPanel } from '../../components/ModelSelectionPanel.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import { EngineSettingsPanel } from '../../components/EngineSettingsPanel.tsx';
import type { Config, DownloadPhase, EngineCapabilities, GpuStatus, ModelInfo } from '../../types.ts';
import { inputBaseStyle, selectWrapperStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';

interface TranscriptionSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  isDownloading: boolean;
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  gpuStatus: GpuStatus | null;
  engineCapabilities: EngineCapabilities | null;
  testApiKey: () => void;
  isTestingApi: boolean;
  downloadModel: (size: string, engine?: string) => void;
  loadModels: () => void;
  setShowModelGuide: (show: boolean) => void;
  languageOptions: { value: string; label: string }[];
}

export function TranscriptionSection({
  config,
  updateConfig,
  availableEngines,
  availableModels,
  modelStatus,
  isDownloading,
  downloadProgress,
  downloadPhase,
  gpuStatus,
  engineCapabilities,
  testApiKey,
  isTestingApi,
  downloadModel,
  loadModels,
  setShowModelGuide,
  languageOptions,
}: TranscriptionSectionProps) {
  return (
    <>
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
              <input
                style={inputBaseStyle}
                type="text"
                value={config.openai_api_key}
                onChange={(e: Event) => updateConfig('openai_api_key', (e.target as HTMLInputElement).value)}
                placeholder="sk-..."
              />
              <Button variant="configAction" onClick={testApiKey} disabled={isTestingApi}>
                {isTestingApi ? '...' : 'Test'}
              </Button>
            </div>
          </ConfigField>

          <ConfigField label="API URL" description="The endpoint that processes audio (OpenAI or Local Whisper).">
            <input
              style={inputBaseStyle}
              type="url"
              value={config.api_url}
              onChange={(e: Event) => updateConfig('api_url', (e.target as HTMLInputElement).value)}
            />
          </ConfigField>

          <ConfigField label="API Model" description="The model name to use with the API provider.">
            <input
              style={inputBaseStyle}
              type="text"
              value={config.api_model}
              onChange={(e: Event) => updateConfig('api_model', (e.target as HTMLInputElement).value)}
            />
          </ConfigField>
        </>
      ) : (
        <>
          <ConfigField label="Local Engine" description="The core technology used to process your voice locally.">
            <div style={selectWrapperStyle}>
              <SelectField
                value={config.local_engine}
                options={availableEngines.map((engine) => ({ value: engine, label: engine }))}
                onChange={(nextEngine) => {
                  updateConfig('local_engine', nextEngine);
                  const modelsForEngine = availableModels.filter((m) => m.engine === nextEngine);
                  if (modelsForEngine.length > 0) {
                    const recommended = modelsForEngine.find((m) => m.recommended) || modelsForEngine[0];
                    updateConfig('local_model_size', recommended.size);
                  }
                }}
                ariaLabel="Local engine"
              />
            </div>
          </ConfigField>

          {config.local_engine.includes('(GPU)') && gpuStatus?.tested && !gpuStatus.available && (
            <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              fontSize: '11px',
              color: '#f1c40f',
              margin: '-8px 0 8px',
              padding: '6px 8px',
              background: 'rgba(241, 196, 15, 0.08)',
              borderRadius: '6px',
            }}>
              <IconInfoCircle size={14} />
              <span>
                GPU unavailable: {gpuStatus.detail || 'No compatible GPU detected. Select Whisper.cpp (CPU) or check your GPU drivers.'}
              </span>
            </div>
          )}

          <ConfigField label="Local Model" description="Choose the Whisper model size. Distil-Small is recommended for most users.">
            <ModelSelectionPanel
              availableModels={availableModels}
              localEngine={config.local_engine}
              localModelSize={config.local_model_size}
              modelStatus={modelStatus}
              isDownloading={isDownloading}
              downloadProgress={downloadProgress}
              downloadPhase={downloadPhase}
              onChangeModel={(size) => updateConfig('local_model_size', size)}
              onShowModelGuide={() => setShowModelGuide(true)}
              onDownloadModel={downloadModel}
              onRetryModels={loadModels}
            />
          </ConfigField>

          {engineCapabilities && engineCapabilities.settings.length > 0 && (
            <ConfigField label="Engine Settings" description="Fine-tune how this engine processes your speech.">
              <EngineSettingsPanel
                capabilities={engineCapabilities}
                values={(config.engine_config || {})}
                onChange={(key, value) => {
                  const current = { ...(config.engine_config || {}) };
                  (current as Record<string, unknown>)[key] = value;
                  updateConfig('engine_config', current);
                }}
              />
            </ConfigField>
          )}
        </>
      )}

      <ConfigField label="Language Hint" labelBadge="Experimental" description="Best-effort language hint for transcription models. Helps with spelling preferences (e.g. Australian, British, American English) and non-English dictation.">
        <div style={selectWrapperStyle}>
          <SelectField
            value={config.language}
            options={languageOptions}
            onChange={(nextLanguage) => updateConfig('language', nextLanguage)}
            ariaLabel="Language hint"
          />
        </div>
      </ConfigField>

      <ConfigField label="Differentiate Voices in Recordings" description="Detect and label different speakers in live recordings. When enabled, the recording takes slightly longer to process as each speaker segment is transcribed independently.">
        <Switch name="Diarization Recording" checked={config.diarization_enabled_recording} onChange={(checked) => updateConfig('diarization_enabled_recording', checked)} />
      </ConfigField>

      {(config.diarization_enabled_recording || config.diarization_enabled_files) && (
        <ConfigField label="Voice Distinctiveness" description="Higher values merge similar voices (fewer speaker labels); lower values detect more distinct voices. Adjust if you're seeing too many or too few speakers.">
          <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
            <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, textAlign: 'left' }}>
              {Math.round(config.diarization_cluster_threshold * 100)}%
            </div>
            <SliderField
              value={config.diarization_cluster_threshold}
              min={0.3}
              max={0.95}
              step={0.05}
              formatEndLabel={(value) => `${Math.round(value * 100)}%`}
              onChange={(value) => updateConfig('diarization_cluster_threshold', value)}
              ariaLabel="Voice distinctiveness"
              style={{ margin: `${tokens.spacing.sm} 0` }}
            />
          </div>
        </ConfigField>
      )}

      <ConfigField label="Post-roll (ms)" description="Extra audio (in milliseconds) captured after releasing the hotkey. Helps prevent the last sentence from being cut off, especially with API models.">
        <NumberField
          value={config.post_roll_ms}
          onChange={(value) => updateConfig('post_roll_ms', value)}
          min={0}
          max={2000}
          step={50}
        />
      </ConfigField>

      <ConfigField label="Max Recording Length (minutes)" description="Recording automatically stops and transcribes after this many minutes (1-180).">
        <NumberField
          value={config.max_recording_duration_minutes}
          onChange={(value) => updateConfig('max_recording_duration_minutes', value)}
          min={1}
          max={180}
        />
      </ConfigField>
    </>
  );
}
