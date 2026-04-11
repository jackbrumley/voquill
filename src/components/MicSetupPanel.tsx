import { Button } from './Button.tsx';

interface MicSetupPanelProps {
  inputSensitivity: number;
  onInputSensitivityChange: (value: number) => void;
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
  compact?: boolean;
  actionButtonSize?: 'sm' | 'md' | 'lg';
}

export function MicSetupPanel({
  inputSensitivity,
  onInputSensitivityChange,
  micTestStatus,
  micVolume,
  onStartMicTest,
  onStopMicTest,
  onStopMicPlayback,
  compact = false,
  actionButtonSize = 'md',
}: MicSetupPanelProps) {
  const showVolumeMeter = micTestStatus === 'recording' || micTestStatus === 'playing';

  return (
    <div className={`mic-setup-panel ${compact ? 'compact' : ''}`}>
      <div className="mic-sensitivity-label">
        Mic Sensitivity ({Math.round(inputSensitivity * 100)}%)
      </div>
      <input
        type="range"
        min="0.1"
        max="2.0"
        step="0.05"
        value={inputSensitivity}
        onChange={(event: Event) => {
          const target = event.target as HTMLInputElement;
          onInputSensitivityChange(parseFloat(target.value));
        }}
        className="slider"
      />

      <div className="mic-test-row">
        {showVolumeMeter && (
          <div className="volume-meter-container">
            <div
              className={`volume-meter-bar ${micVolume > 0.9 ? 'clipping' : micVolume > 0.7 ? 'warning' : ''}`}
              style={{ width: `${Math.min(micVolume * 100, 100)}%` }}
            ></div>
          </div>
        )}

        <Button
          disabled={micTestStatus === 'processing'}
          variant="configAction"
          size={actionButtonSize}
          onClick={() => {
            if (micTestStatus === 'idle') {
              onStartMicTest();
            } else if (micTestStatus === 'recording') {
              onStopMicTest();
            } else if (micTestStatus === 'playing') {
              onStopMicPlayback();
            }
          }}
        >
          {micTestStatus === 'idle'
            ? 'Test Microphone'
            : micTestStatus === 'recording'
              ? 'Stop & Play Back'
              : micTestStatus === 'playing'
                ? 'Stop Playback'
                : 'Processing...'}
        </Button>
      </div>
    </div>
  );
}
