import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { NumberField } from '../../components/NumberField.tsx';
import type { Config } from '../../types.ts';

interface TypingSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
}

export function TypingSection({ config, updateConfig }: TypingSectionProps) {
  return (
    <>
      <ConfigField label="Typing Speed (ms / char)" description="Delay between simulated key presses. Lower is faster (1ms recommended); raise if characters drop in slow applications.">
        <NumberField
          value={config.typing_speed_interval}
          min={1}
          max={50}
          onChange={(value) => updateConfig('typing_speed_interval', value)}
        />
      </ConfigField>

      <ConfigField label="Key Hold Duration (ms)" description="How long each virtual key is held down before release.">
        <NumberField
          value={config.key_press_duration_ms}
          onChange={(value) => updateConfig('key_press_duration_ms', value)}
          min={1}
          max={50}
        />
      </ConfigField>

      <ConfigField label="Append Trailing Space" description="Automatically append a space after typed text so you're ready to type the next word.">
        <Switch name="Append Trailing Space" checked={config.append_trailing_space} onChange={(checked) => updateConfig('append_trailing_space', checked)} />
      </ConfigField>

      <ConfigField label="Auto-Submit with Enter" description="Automatically press Enter after dictation finishes to submit the message or command.">
        <Switch name="Auto-Submit" checked={config.auto_submit} onChange={(checked) => updateConfig('auto_submit', checked)} />
      </ConfigField>
    </>
  );
}
