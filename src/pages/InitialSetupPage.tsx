import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { Button } from '../components/Button.tsx';
import { helperTextStyle, tabPanelPaddedStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import type { ReadinessStatus } from '../readiness.ts';
import type { AudioDevice, DownloadPhase, GpuStatus, EngineInfo, ModelInfo } from '../types.ts';
import { useSetupRecommendation } from '../hooks/useSetupRecommendation.ts';
import { useSetupSteps } from '../hooks/useSetupSteps.ts';
import { SetupStep } from './setup/SetupStep.tsx';
import { PermissionsStep } from './setup/PermissionsStep.tsx';
import { MicrophoneStep, microphoneSummary } from './setup/MicrophoneStep.tsx';
import { SpeechEngineStep, speechEngineSummary } from './setup/SpeechEngineStep.tsx';
import { TryItStep } from './setup/TryItStep.tsx';
import type { PlatformPermissions, SetupConfig, SetupReview } from './setup/setupTypes.ts';

interface InitialSetupPageProps {
  permissions: PlatformPermissions | null;
  config: SetupConfig;
  readiness: ReadinessStatus;
  availableEngines: EngineInfo[];
  availableModels: ModelInfo[];
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  isDownloading: boolean;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  isApplyingHotkey: boolean;
  hotkeyError: string | null;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  isMicTriggered?: boolean;
  isMicTooQuiet: boolean;
  gpuStatus: GpuStatus | null;
  isTestingEngine: boolean;
  isTestingApi: boolean;
  isRecordingHotkey: boolean;
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
  onTestApiKey: () => void;
  /// Set when setup opened to review new options after an update.
  setupReview: SetupReview | null;
  /// Records setup as complete and opens Home.
  onFinishSetup: () => void;
  autostartEnabled: boolean;
  onEnableAutostart: () => Promise<void>;
}

/// First-run setup as numbered steps: permissions, microphone, speech engine,
/// then a real dictation to prove it all works. Only the current step is
/// open; finished steps fold to a tick.
export function InitialSetupPage(props: InitialSetupPageProps) {
  const { config, readiness, setupReview } = props;
  const isReviewing = setupReview !== null;
  const steps = useSetupSteps(readiness, isReviewing ? 'speech' : null);
  const { recommendation, error: recommendationError } = useSetupRecommendation();
  // The engine the user had when the review opened, restored by "Keep my
  // current setup" after they tried other choices.
  const originalEngine = useSignal({ engine: config.local_engine, modelSize: config.local_model_size });
  const originalModelLabel = props.availableModels.find(
    (model) => model.engine === originalEngine.value.engine && model.size === originalEngine.value.modelSize,
  )?.label ?? originalEngine.value.modelSize;

  // Ticked by default: a dictation tool is most useful when it is always
  // running, but the choice stays visible rather than being made silently.
  const startAtLogin = useSignal(true);
  const finishSetup = async () => {
    if (!isReviewing && startAtLogin.value && !props.autostartEnabled) {
      await props.onEnableAutostart();
    }
    props.onFinishSetup();
  };

  // Restoring the original engine takes a render to reach readiness, so
  // setup finishes once the restored setup reports ready.
  const isKeepingCurrentSetup = useSignal(false);
  const keepCurrentSetup = () => {
    props.onSelectEngine(originalEngine.value.engine);
    props.onChangeConfig('local_model_size', originalEngine.value.modelSize);
    isKeepingCurrentSetup.value = true;
  };
  useEffect(() => {
    if (isKeepingCurrentSetup.value && readiness.isAllReady) {
      isKeepingCurrentSetup.value = false;
      props.onFinishSetup();
    }
  }, [isKeepingCurrentSetup.value, readiness.isAllReady]);

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto' }} key="initial-setup">
      <div style={{ ...tabPanelPaddedStyle, flex: 1, paddingTop: tokens.spacing.md }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <h2 style={{ margin: 0, fontSize: '22px', fontWeight: 800, letterSpacing: '-0.02em', color: tokens.colors.textPrimary }}>
            {reviewHeading(setupReview)}
          </h2>
          <p style={{ ...helperTextStyle, margin: 0 }}>
            {reviewIntroduction(setupReview, originalModelLabel)}
          </p>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
          <SetupStep
            number={1}
            title="Permissions"
            summary="Granted"
            state={steps.stateOf('permissions')}
            isOpen={steps.isOpen('permissions')}
            onToggle={() => steps.toggle('permissions')}
          >
            <PermissionsStep
              permissions={props.permissions}
              config={config}
              isHotkeyReady={readiness.isHotkeyReady}
              hotkeyError={props.hotkeyError}
              portalVersion={props.portalVersion}
              isSystemManagedShortcut={props.isSystemManagedShortcut}
              isApplyingHotkey={props.isApplyingHotkey}
              isRecordingHotkey={props.isRecordingHotkey}
              onAudioSetup={props.onAudioSetup}
              onInputSetup={props.onInputSetup}
              onConfigureHotkey={props.onConfigureHotkey}
              onHotkeyKeyDown={props.onHotkeyKeyDown}
              onHotkeyKeyUp={props.onHotkeyKeyUp}
              onHotkeyBlur={props.onHotkeyBlur}
            />
          </SetupStep>

          <SetupStep
            number={2}
            title="Microphone"
            summary={microphoneSummary(config, props.availableMics)}
            state={steps.stateOf('microphone')}
            isOpen={steps.isOpen('microphone')}
            onToggle={() => steps.toggle('microphone')}
          >
            <MicrophoneStep
              config={config}
              audioDeviceIssue={readiness.audioDeviceIssue}
              availableMics={props.availableMics}
              micTestStatus={props.micTestStatus}
              micVolume={props.micVolume}
              isMicTriggered={props.isMicTriggered}
              isMicTooQuiet={props.isMicTooQuiet}
              onChangeConfig={props.onChangeConfig}
              onLoadMics={props.onLoadMics}
              onStartMicTest={props.onStartMicTest}
              onStopMicTest={props.onStopMicTest}
              onStopMicPlayback={props.onStopMicPlayback}
            />
          </SetupStep>

          <SetupStep
            number={3}
            title="Speech engine"
            summary={speechEngineSummary(config, recommendation)}
            state={steps.stateOf('speech')}
            isOpen={steps.isOpen('speech')}
            onToggle={() => steps.toggle('speech')}
          >
            <SpeechEngineStep
              config={config}
              recommendation={recommendation}
              recommendationError={recommendationError}
              reviewChoice={setupReview?.recommended ?? null}
              availableEngines={props.availableEngines}
              availableModels={props.availableModels}
              isDownloading={props.isDownloading}
              downloadProgress={props.downloadProgress}
              downloadPhase={props.downloadPhase}
              gpuStatus={props.gpuStatus}
              isTestingEngine={props.isTestingEngine}
              isTestingApi={props.isTestingApi}
              onSelectEngine={props.onSelectEngine}
              onChangeConfig={props.onChangeConfig}
              onDownloadModel={props.onDownloadModel}
              onShowModelGuide={props.onShowModelGuide}
              onRetryModels={props.onRetryModels}
              onTestApiKey={props.onTestApiKey}
            />
          </SetupStep>

          <SetupStep
            number={4}
            title="Try it"
            state={steps.stateOf('try')}
            isOpen={steps.isOpen('try')}
          >
            <TryItStep config={config} />
          </SetupStep>
        </div>

        {readiness.postProcessModelMissing && (
          <div style={{ ...helperTextStyle, color: '#f1c40f', display: 'flex', alignItems: 'center', gap: tokens.spacing.sm }}>
            <span style={{ flex: 1 }}>Text cleanup is turned on, but its model isn't downloaded yet.</span>
            <Button
              variant="secondary"
              size="sm"
              pill
              disabled={props.isDownloading}
              onClick={() => props.onDownloadPostProcessModel(config.post_process_model)}
            >
              Download
            </Button>
          </div>
        )}

        {!isReviewing && !props.autostartEnabled && (
          <label style={{ display: 'inline-flex', alignItems: 'center', gap: tokens.spacing.sm, fontSize: tokens.typography.sizeSm, color: tokens.colors.textSecondary, cursor: 'pointer', alignSelf: 'flex-start' }}>
            <input
              type="checkbox"
              checked={startAtLogin.value}
              onChange={(event) => { startAtLogin.value = (event.target as HTMLInputElement).checked; }}
              style={{ accentColor: tokens.colors.accentPrimary, cursor: 'pointer' }}
            />
            Start Voquill when I log in
          </label>
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-start', alignItems: 'center', gap: tokens.spacing.sm, flexWrap: 'wrap', paddingBottom: tokens.spacing.xl }}>
          <Button
            variant="configAction"
            disabled={!readiness.isAllReady}
            onClick={() => void finishSetup()}
            style={{ minWidth: '160px' }}
          >
            {readiness.isAllReady ? 'Start using Voquill' : 'Finish the steps above'}
          </Button>
          {offersBetterChoice(setupReview) && (
            <Button variant="ghost" pill onClick={keepCurrentSetup}>
              Keep my current setup
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

/// The backend recommended a different choice than the one the user runs.
function offersBetterChoice(review: SetupReview | null): boolean {
  return review !== null && review.recommended !== null && !review.already_recommended;
}

function reviewHeading(review: SetupReview | null): string {
  if (review === null) {
    return 'Set up Voquill';
  }
  return offersBetterChoice(review) ? 'Voquill has new options for your computer' : 'Voquill has been updated';
}

function reviewIntroduction(review: SetupReview | null, currentModelLabel: string): string {
  if (review === null) {
    return "A few quick steps and you're ready to dictate.";
  }
  if (offersBetterChoice(review)) {
    return `You're using ${currentModelLabel}. Voquill found a choice that should be faster or more accurate. Your current setup still works.`;
  }
  if (review.already_recommended) {
    return `You're using ${currentModelLabel}, the best choice for this computer. Check your setup below, then carry on.`;
  }
  return 'Check your setup below, then carry on.';
}
