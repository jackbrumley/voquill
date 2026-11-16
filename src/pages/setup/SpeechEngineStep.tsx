import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { IconCheck } from '@tabler/icons-preact';
import { Button } from '../../components/Button.tsx';
import { DownloadProgressBar } from '../../components/DownloadProgressBar.tsx';
import { ModelSelectionPanel } from '../../components/ModelSelectionPanel.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import { RatingDots } from '../../components/RatingDots.tsx';
import { helperTextStyle, inputBaseStyle, selectWrapperStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import { API_KEY_PLACEHOLDER } from '../../readiness.ts';
import { engineUsesGpu, enginesInCategory, isModelDownloaded } from '../../models.ts';
import type { DownloadPhase, GpuStatus, EngineInfo, ModelInfo } from '../../types.ts';
import type { SetupConfig, SetupOption, SetupRecommendation } from './setupTypes.ts';

interface SpeechEngineStepProps {
  config: SetupConfig;
  recommendation: SetupRecommendation | null;
  recommendationError: string | null;
  /// The backend's recommended choice when setup opened as an after-update
  /// review. Setup starts from it; "Keep my current setup" restores the
  /// original engine.
  reviewChoice: SetupOption | null;
  availableEngines: EngineInfo[];
  availableModels: ModelInfo[];
  isDownloading: boolean;
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  gpuStatus: GpuStatus | null;
  isTestingEngine: boolean;
  isTestingApi: boolean;
  onSelectEngine: (engine: string) => void;
  onChangeConfig: (key: string, value: string | number | boolean | null) => void;
  onDownloadModel: (size: string) => void;
  onShowModelGuide: () => void;
  onRetryModels: () => void;
  onTestApiKey: () => void;
}

const warningTextStyle = { ...helperTextStyle, color: '#f1c40f' } as const;
const linkButtonStyle = {
  alignSelf: 'flex-start',
  border: 'none',
  background: 'transparent',
  padding: 0,
  color: '#818cf8',
  fontSize: tokens.typography.sizeXs,
  fontWeight: 600,
  cursor: 'pointer',
} as const;

function megabytes(bytes: number): string {
  return `${Math.round(bytes / 1024 / 1024)} MB`;
}

function isSelected(option: SetupOption, config: SetupConfig): boolean {
  return option.engine === config.local_engine && option.model_size === config.local_model_size;
}

/// The label of the configured speech engine, for the folded step summary.
export function speechEngineSummary(config: SetupConfig, recommendation: SetupRecommendation | null): string {
  if (config.transcription_mode === 'API') {
    return 'Cloud transcription';
  }
  const chosen = recommendation?.options.find((option) => isSelected(option, config));
  return chosen?.title ?? `${config.local_engine} · ${config.local_model_size}`;
}

/// Step 3: choose and download the speech engine. The backend recommends
/// plain choices for this computer; every model stays one link away.
export function SpeechEngineStep(props: SpeechEngineStepProps) {
  const { config, recommendation, availableModels } = props;
  const isOtherModelsOpen = useSignal(false);
  const hasAppliedRecommendation = useSignal(false);
  const isModelStatusKnown = availableModels.length > 0;

  // Once, start from the backend's first choice, unless the configured
  // model is already downloaded or is one of the offered choices. Waits for
  // the download status so a working setup is never switched away from.
  // A review starts from the choice it was opened for: offering something
  // better is its whole purpose.
  useEffect(() => {
    const options = recommendation?.options ?? [];
    if (hasAppliedRecommendation.value || !isModelStatusKnown || options.length === 0) {
      return;
    }
    hasAppliedRecommendation.value = true;
    if (config.transcription_mode !== 'Local') {
      return;
    }
    if (props.reviewChoice) {
      chooseOption(props.reviewChoice);
      return;
    }
    const alreadyChosen = options.some((option) => isSelected(option, config));
    if (!alreadyChosen && !isModelDownloaded(availableModels, config.local_engine, config.local_model_size)) {
      chooseOption(options[0]);
    }
  }, [recommendation, isModelStatusKnown]);

  const chooseOption = (option: SetupOption) => {
    props.onSelectEngine(option.engine);
    props.onChangeConfig('local_model_size', option.model_size);
  };

  if (config.transcription_mode === 'API') {
    return (
      <>
        <div style={helperTextStyle}>Enter your OpenAI API key to transcribe in the cloud.</div>
        <div style={selectWrapperStyle}>
          <input
            style={inputBaseStyle}
            type="text"
            value={config.openai_api_key === API_KEY_PLACEHOLDER ? '' : config.openai_api_key}
            onChange={(event: Event) => props.onChangeConfig('openai_api_key', (event.target as HTMLInputElement).value)}
            placeholder="sk-..."
            aria-label="API key"
          />
          <Button variant="configAction" size="sm" onClick={props.onTestApiKey} disabled={props.isTestingApi}>
            {props.isTestingApi ? '...' : 'Test'}
          </Button>
        </div>
      </>
    );
  }

  const isGpuEngine = engineUsesGpu(props.availableEngines, config.local_engine);
  const isModelReady = isModelDownloaded(availableModels, config.local_engine, config.local_model_size);

  return (
    <>
      <div style={helperTextStyle}>
        Turns your voice into text on this computer; nothing leaves your machine. Voquill picks the engine that suits
        your computer, so faster computers, especially ones with a graphics card, get faster and more accurate results.
      </div>
      {recommendation?.explanation && (
        <div style={{ ...helperTextStyle, color: tokens.colors.textPrimary }}>{recommendation.explanation}</div>
      )}

      {props.recommendationError && (
        <div style={warningTextStyle}>Couldn't check your hardware: {props.recommendationError}</div>
      )}

      {(recommendation?.options ?? []).map((option) => {
        const selected = isSelected(option, config);
        return (
          <button
            key={`${option.engine}/${option.model_size}`}
            type="button"
            onClick={() => chooseOption(option)}
            style={{
              display: 'flex',
              alignItems: 'flex-start',
              gap: '10px',
              textAlign: 'left',
              padding: '10px 12px',
              borderRadius: '10px',
              cursor: 'pointer',
              color: 'inherit',
              background: selected ? 'rgba(88, 101, 242, 0.12)' : 'rgba(255, 255, 255, 0.03)',
              border: `1px solid ${selected ? 'rgba(88, 101, 242, 0.6)' : 'rgba(255, 255, 255, 0.08)'}`,
            }}
          >
            <span style={{
              width: '14px',
              height: '14px',
              marginTop: '2px',
              borderRadius: '50%',
              flexShrink: 0,
              border: `2px solid ${selected ? tokens.colors.accentPrimary : tokens.colors.textMuted}`,
              background: selected ? tokens.colors.accentPrimary : 'transparent',
              boxShadow: selected ? 'inset 0 0 0 2px #151822' : 'none',
            }} />
            <span style={{ display: 'flex', flexDirection: 'column', flex: 1, minWidth: 0 }}>
              <span style={{ display: 'flex', justifyContent: 'space-between', gap: tokens.spacing.sm }}>
                <span style={{ fontSize: tokens.typography.sizeSm, fontWeight: 600, color: tokens.colors.textPrimary }}>{option.title}</span>
                <span style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, whiteSpace: 'nowrap' }}>{megabytes(option.download_bytes)}</span>
              </span>
              <span style={helperTextStyle}>{option.description}</span>
              <span style={{ display: 'flex', gap: tokens.spacing.md, marginTop: '4px' }}>
                <RatingDots label="Speed" value={option.speed} />
                <RatingDots label="Accuracy" value={option.accuracy} />
              </span>
            </span>
          </button>
        );
      })}

      {!isOtherModelsOpen.value && (isModelReady ? (
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: tokens.typography.sizeSm, color: tokens.colors.success }}>
          <IconCheck size={16} /> Downloaded and ready
        </div>
      ) : props.isDownloading ? (
        <DownloadProgressBar isDownloading progress={props.downloadProgress} phase={props.downloadPhase} compact />
      ) : (
        <Button variant="configAction" size="sm" style={{ alignSelf: 'flex-start' }} onClick={() => props.onDownloadModel(config.local_model_size)}>
          Download
        </Button>
      ))}

      {isGpuEngine && isModelReady && props.isTestingEngine && (
        <div style={helperTextStyle}>Checking your graphics card…</div>
      )}
      {isGpuEngine && isModelReady && !props.isTestingEngine && props.gpuStatus?.tested && !props.gpuStatus.available && (
        <div style={warningTextStyle}>
          Your graphics card couldn't be used ({props.gpuStatus.detail || 'no compatible graphics card found'}), so speech will be slower. You can choose "Works on any computer" instead.
        </div>
      )}

      <button type="button" style={linkButtonStyle} onClick={() => { isOtherModelsOpen.value = !isOtherModelsOpen.value; }}>
        {isOtherModelsOpen.value ? 'Hide other models' : 'Choose a different model ›'}
      </button>
      {isOtherModelsOpen.value && (
        <>
          <SelectField
            value={config.local_engine}
            onChange={props.onSelectEngine}
            searchable={false}
            ariaLabel="Speech engine"
            options={enginesInCategory(props.availableEngines, 'transcription').map((engine) => ({ value: engine.name, label: engine.name }))}
          />
          <ModelSelectionPanel
            availableModels={props.availableModels}
            localEngine={config.local_engine}
            localModelSize={config.local_model_size}
            isDownloading={props.isDownloading}
            downloadProgress={props.downloadProgress}
            downloadPhase={props.downloadPhase}
            actionButtonSize="sm"
            onChangeModel={(size) => props.onChangeConfig('local_model_size', size)}
            onShowModelGuide={props.onShowModelGuide}
            onDownloadModel={props.onDownloadModel}
            onRetryModels={props.onRetryModels}
          />
        </>
      )}
    </>
  );
}
