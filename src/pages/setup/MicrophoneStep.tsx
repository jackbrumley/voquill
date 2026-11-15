import { useSignal } from '@preact/signals';
import { IconRefresh } from '@tabler/icons-preact';
import { Button } from '../../components/Button.tsx';
import { MicSetupPanel } from '../../components/MicSetupPanel.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import { helperTextStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import type { AudioDeviceIssue } from '../../readiness.ts';
import type { AudioDevice } from '../../types.ts';
import type { SetupConfig } from './setupTypes.ts';

interface MicrophoneStepProps {
  config: SetupConfig;
  audioDeviceIssue: AudioDeviceIssue;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  isMicTriggered?: boolean;
  isMicTooQuiet: boolean;
  onChangeConfig: (key: string, value: string | number | boolean | null) => void;
  onLoadMics: () => void;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
}

const warningTextStyle = { ...helperTextStyle, color: '#f1c40f' } as const;

/// The label of the configured microphone, for the folded step summary.
export function microphoneSummary(config: SetupConfig, availableMics: AudioDevice[]): string {
  const selectedId = config.audio_device || 'default';
  const selected = availableMics.find((mic) => mic.id === selectedId);
  return selected?.label || selected?.id || 'System default';
}

/// Step 2: pick the microphone. Testing it is optional and folded away.
export function MicrophoneStep(props: MicrophoneStepProps) {
  const { config, audioDeviceIssue, availableMics } = props;
  const isTestOpen = useSignal(false);

  return (
    <>
      {audioDeviceIssue === 'no-devices' && (
        <div style={warningTextStyle}>No microphone found. Plug one in, then press Refresh.</div>
      )}
      {audioDeviceIssue === 'device-missing' && (
        <div style={warningTextStyle}>The microphone you chose before isn't connected. Choose another one.</div>
      )}

      <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.sm }}>
        {audioDeviceIssue !== 'no-devices' && (
          <div style={{ flex: 1, minWidth: 0 }}>
            <SelectField
              value={config.audio_device || 'default'}
              onChange={(nextMicId) => props.onChangeConfig('audio_device', nextMicId)}
              options={availableMics.map((mic) => ({ value: mic.id, label: mic.label || mic.id }))}
              ariaLabel="Microphone"
            />
          </div>
        )}
        <Button variant="ghost" size="sm" pill onClick={props.onLoadMics} title="Look for microphones again">
          <IconRefresh size={14} />
          Refresh
        </Button>
      </div>

      {audioDeviceIssue === 'none' && (
        <button
          type="button"
          onClick={() => { isTestOpen.value = !isTestOpen.value; }}
          style={{
            alignSelf: 'flex-start',
            border: 'none',
            background: 'transparent',
            padding: 0,
            color: '#818cf8',
            fontSize: tokens.typography.sizeXs,
            fontWeight: 600,
            cursor: 'pointer',
          }}
        >
          {isTestOpen.value ? 'Hide microphone test' : 'Test your microphone ›'}
        </button>
      )}
      {isTestOpen.value && (
        <MicSetupPanel
          compact
          inputSensitivity={config.input_sensitivity}
          actionButtonSize="sm"
          onInputSensitivityChange={(value) => props.onChangeConfig('input_sensitivity', value)}
          micTestStatus={props.micTestStatus}
          micVolume={props.micVolume}
          isMicTriggered={props.isMicTriggered}
          isMicTooQuiet={props.isMicTooQuiet}
          onStartMicTest={props.onStartMicTest}
          onStopMicTest={props.onStopMicTest}
          onStopMicPlayback={props.onStopMicPlayback}
        />
      )}
    </>
  );
}
