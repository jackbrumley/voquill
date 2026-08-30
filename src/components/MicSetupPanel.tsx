import { Button } from './Button.tsx';
import { SliderField } from './SliderField.tsx';
import { tokens } from '../design-tokens.ts';

interface MicSetupPanelProps {
  inputSensitivity: number;
  onInputSensitivityChange: (value: number) => void;
  voiceMacroActivationThreshold?: number;
  onVoiceMacroActivationThresholdChange?: (value: number) => void;
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  isMicTriggered?: boolean;
  onStartMicTest: () => void;
  onStopMicTest: () => void;
  onStopMicPlayback: () => void;
  compact?: boolean;
  actionButtonSize?: 'sm' | 'md' | 'lg';
}

export function MicSetupPanel({
  inputSensitivity,
  onInputSensitivityChange,
  voiceMacroActivationThreshold,
  onVoiceMacroActivationThresholdChange,
  micTestStatus,
  micVolume,
  isMicTriggered = false,
  onStartMicTest,
  onStopMicTest,
  onStopMicPlayback,
  compact = false,
  actionButtonSize = 'md',
}: MicSetupPanelProps) {
  const showVolumeMeter = micTestStatus === 'recording';
  const showPlaybackText = micTestStatus === 'playing';

  const threshold = voiceMacroActivationThreshold ?? 0.035;
  const isTriggered = isMicTriggered;
  // Map volume and threshold to 0-100% on meter (scale max 0.15)
  const volPercent = Math.min(Math.max((micVolume / 0.15) * 100, 0), 100);
  const thresholdPercent = Math.min(Math.max((threshold / 0.15) * 100, 2), 98);

  return (
    <div style={{ width: '100%' }}>
      <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, marginBottom: '4px', textAlign: 'left' }}>
        Mic Sensitivity / Gain ({Math.round(inputSensitivity * 100)}%)
      </div>
      <SliderField
        value={inputSensitivity}
        min={0.1}
        max={2.0}
        step={0.05}
        formatEndLabel={(v) => `${Math.round(v * 100)}%`}
        onChange={onInputSensitivityChange}
        ariaLabel="Mic sensitivity"
        style={{ margin: `${tokens.spacing.sm} 0` }}
      />

      {voiceMacroActivationThreshold !== undefined && onVoiceMacroActivationThresholdChange && (
        <div style={{ marginTop: tokens.spacing.sm }}>
          <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, marginBottom: '4px', textAlign: 'left' }}>
            Voice Activation Threshold ({(threshold * 100).toFixed(1)}%)
          </div>
          <SliderField
            value={voiceMacroActivationThreshold}
            min={0.01}
            max={0.12}
            step={0.005}
            formatEndLabel={(v) => `${(v * 100).toFixed(1)}%`}
            onChange={onVoiceMacroActivationThresholdChange}
            ariaLabel="Voice activation threshold"
            style={{ margin: `${tokens.spacing.sm} 0` }}
          />
        </div>
      )}

      <div
        style={{
          marginTop: compact ? tokens.spacing.sm : tokens.spacing.md,
          display: 'flex',
          flexDirection: 'column',
          gap: tokens.spacing.sm,
          alignItems: 'center',
        }}
      >
        <div style={{ width: '100%', minHeight: '24px', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '4px' }}>
          {showVolumeMeter && (
            <div style={{ width: '100%', display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <div
                style={{
                  position: 'relative',
                  width: '100%',
                  height: '8px',
                  background: 'rgba(255, 255, 255, 0.08)',
                  borderRadius: '4px',
                  overflow: 'hidden',
                }}
              >
                {/* Volume Level Fill */}
                <div
                  style={{
                    width: `${volPercent}%`,
                    height: '100%',
                    background: isTriggered
                      ? 'linear-gradient(90deg, #22c55e 0%, #4ade80 100%)'
                      : 'rgba(255, 255, 255, 0.35)',
                    boxShadow: isTriggered ? '0 0 10px rgba(34, 197, 94, 0.7)' : 'none',
                    transition: 'width 0.06s ease-out, background 0.1s ease',
                  }}
                />

                {/* Threshold Notch */}
                <div
                  style={{
                    position: 'absolute',
                    top: 0,
                    bottom: 0,
                    left: `${thresholdPercent}%`,
                    width: '2px',
                    background: '#ffffff',
                    boxShadow: '0 0 4px rgba(255, 255, 255, 0.9)',
                    zIndex: 2,
                  }}
                  title="Trigger Threshold Notch"
                />
              </div>

              {/* Live Status Indicator */}
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <div
                    style={{
                      width: '8px',
                      height: '8px',
                      borderRadius: '50%',
                      background: isTriggered ? '#22c55e' : 'rgba(255, 255, 255, 0.2)',
                      boxShadow: isTriggered ? '0 0 8px #22c55e' : 'none',
                      transition: 'all 0.08s ease',
                    }}
                  />
                  <span
                    style={{
                      fontSize: '11px',
                      fontWeight: 600,
                      color: isTriggered ? '#22c55e' : tokens.colors.textMuted,
                      letterSpacing: '0.4px',
                      transition: 'color 0.08s ease',
                    }}
                  >
                    {isTriggered ? 'VOICE ACTIVATED' : 'LISTENING (BELOW THRESHOLD)'}
                  </span>
                </div>
                <span style={{ fontSize: '11px', color: tokens.colors.textMuted, fontFamily: 'monospace' }}>
                  {(micVolume * 100).toFixed(1)}%
                </span>
              </div>
            </div>
          )}

          {showPlaybackText && (
            <span style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textSecondary }}>
              Playing back recording
            </span>
          )}
        </div>

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
