import { useSignal } from '@preact/signals';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { ModeSwitcher } from '../../components/ModeSwitcher.tsx';
import { NumberField } from '../../components/NumberField.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import type { Config, PasteShortcut } from '../../types.ts';
import { helperTextStyle, selectWrapperStyle } from '../../theme/ui-primitives.ts';

interface TypingSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
}

export function TypingSection({ config, updateConfig }: TypingSectionProps) {
  const hasManuallyToggledPaste = useSignal(false);

  return (
    <>
      <ConfigField label="Output Method" description="Choose how transcriptions are inserted when dictation finishes.">
        <ModeSwitcher
          value={config.output_method}
          onToggle={(value) => {
            updateConfig('output_method', value);
            if (value === 'Clipboard' && !hasManuallyToggledPaste.value) {
              updateConfig('paste_after_copy', true);
            }
          }}
          options={[
            { value: 'Clipboard', label: 'Clipboard', title: 'Copy transcription results to your clipboard (Fast & Reliable)' },
            { value: 'Typewriter', label: 'Typewriter', title: 'Type directly into your active cursor via simulated keystrokes' },
          ]}
        />
      </ConfigField>

      {config.output_method === 'Clipboard' && (
        <>
          <ConfigField label="Paste After Copy" description="Automatically paste after copying. Saves and restores your prior clipboard content.">
            <Switch
              name="Paste After Copy"
              checked={config.paste_after_copy}
              onChange={(checked) => {
                hasManuallyToggledPaste.value = true;
                updateConfig('paste_after_copy', checked);
              }}
            />
          </ConfigField>

          {config.paste_after_copy && (
            <ConfigField label="Paste Shortcut" description="Keyboard shortcut simulated to paste your transcription into the active application.">
              <div style={selectWrapperStyle}>
                <SelectField
                  value={config.paste_shortcut}
                  options={[
                    {
                      value: 'ShiftInsert',
                      label: 'Shift + Insert (Universal — Terminal & GUI)',
                    },
                    {
                      value: 'CtrlV',
                      label: 'Ctrl + V (Standard Desktop)',
                    },
                    {
                      value: 'CtrlShiftV',
                      label: 'Ctrl + Shift + V (Terminal Only)',
                    },
                  ]}
                  onChange={(nextShortcut) => updateConfig('paste_shortcut', nextShortcut as PasteShortcut)}
                  ariaLabel="Paste Shortcut"
                />
              </div>
              <div style={helperTextStyle}>
                {config.paste_shortcut === 'ShiftInsert' && (
                  'Shift + Insert is the universal paste shortcut supported by both terminal emulators (e.g. GNOME Terminal, Alacritty, Kitty) and standard desktop applications.'
                )}
                {config.paste_shortcut === 'CtrlV' && (
                  'Ctrl + V is standard across desktop browsers and office apps, but is usually intercepted or ignored in terminal emulators.'
                )}
                {config.paste_shortcut === 'CtrlShiftV' && (
                  'Ctrl + Shift + V is standard in terminal emulators, but is not recognized by standard desktop GUI apps.'
                )}
              </div>
            </ConfigField>
          )}
        </>
      )}

      {config.output_method === 'Typewriter' && (
        <>
          <ConfigField label="Always Copy to Clipboard" description="Also copy transcriptions to clipboard while typing directly into your active cursor.">
            <Switch
              name="Always Copy to Clipboard"
              checked={config.copy_on_typewriter}
              onChange={(checked) => updateConfig('copy_on_typewriter', checked)}
            />
          </ConfigField>

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
        </>
      )}

      <ConfigField label="Append Trailing Space" description="Automatically append a space after typed text so you're ready to type the next word.">
        <Switch name="Append Trailing Space" checked={config.append_trailing_space} onChange={(checked) => updateConfig('append_trailing_space', checked)} />
      </ConfigField>

      <ConfigField label="Auto-Submit with Enter" description="Automatically press Enter after dictation finishes to submit the message or command.">
        <Switch name="Auto-Submit" checked={config.auto_submit} onChange={(checked) => updateConfig('auto_submit', checked)} />
      </ConfigField>
    </>
  );
}
