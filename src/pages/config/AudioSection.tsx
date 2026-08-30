import { IconRefresh } from '@tabler/icons-preact';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { Button } from '../../components/Button.tsx';
import { SliderField } from '../../components/SliderField.tsx';
import { MicSetupPanel } from '../../components/MicSetupPanel.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import type { AudioDevice, Config } from '../../types.ts';
import { selectWrapperStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';

interface AudioSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
  availableMics: AudioDevice[];
  availableSpeakers?: AudioDevice[];
  loadMics: () => void;
  loadSpeakers?: () => void;
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  isMicTriggered?: boolean;
  startMicTest: () => void;
  stopMicTest: () => void;
  stopMicPlayback: () => void;
}

export function AudioSection({
  config,
  updateConfig,
  availableMics,
  availableSpeakers = [],
  loadMics,
  loadSpeakers,
  micTestStatus,
  micVolume,
  isMicTriggered,
  startMicTest,
  stopMicTest,
  stopMicPlayback,
}: AudioSectionProps) {
  return (
    <>
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

      <ConfigField label="Playback Device" description="Choose the output device for mic test playback and history audio listening.">
        <div style={selectWrapperStyle}>
          <SelectField
            value={config.playback_device || 'default'}
            options={availableSpeakers.map((spk) => ({ value: spk.id, label: spk.label }))}
            onChange={(nextSpeakerId) => updateConfig('playback_device', nextSpeakerId)}
            ariaLabel="Playback Device"
          />
          <Button variant="icon" onClick={loadSpeakers || loadMics} title="Refresh Playback Devices">
            <IconRefresh size={16} />
          </Button>
        </div>
      </ConfigField>

      <ConfigField label="Mic Test & Sensitivity" description="Adjust capture gain and verify your microphone playback.">
        <MicSetupPanel
          inputSensitivity={config.input_sensitivity}
          onInputSensitivityChange={(value) => updateConfig('input_sensitivity', value)}
          voiceMacroActivationThreshold={config.voice_macro_activation_threshold}
          onVoiceMacroActivationThresholdChange={(value) => updateConfig('voice_macro_activation_threshold', value)}
          micTestStatus={micTestStatus}
          micVolume={micVolume}
          isMicTriggered={isMicTriggered}
          onStartMicTest={startMicTest}
          onStopMicTest={stopMicTest}
          onStopMicPlayback={stopMicPlayback}
        />
      </ConfigField>

      <ConfigField label="Noise Reduction" description="Reduce background noise from your microphone using spectral gating. Improves transcription accuracy in noisy environments.">
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm, width: '100%' }}>
          <Switch name="Noise Reduction" checked={config.noise_reduction_enabled} onChange={(checked) => updateConfig('noise_reduction_enabled', checked)} />
          {config.noise_reduction_enabled && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
              <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, textAlign: 'left' }}>
                Strength: {Math.round(config.noise_reduction_strength * 100)}%
              </div>
              <SliderField
                value={config.noise_reduction_strength}
                min={0.1}
                max={1.0}
                step={0.05}
                formatEndLabel={(value) => `${Math.round(value * 100)}%`}
                onChange={(value) => updateConfig('noise_reduction_strength', value)}
                ariaLabel="Noise reduction strength"
                style={{ margin: `${tokens.spacing.sm} 0` }}
              />
            </div>
          )}
        </div>
      </ConfigField>
    </>
  );
}
