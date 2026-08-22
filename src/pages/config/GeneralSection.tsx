import { useSignal } from '@preact/signals';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { ModeSwitcher } from '../../components/ModeSwitcher.tsx';
import { Button } from '../../components/Button.tsx';
import { NumberField } from '../../components/NumberField.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import type { Config, PasteShortcut } from '../../types.ts';
import { helperTextStyle, inputBaseStyle, selectWrapperStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';

const configGhostPillStyle = {
  background: 'rgba(255, 255, 255, 0.05)',
  border: '1px solid rgba(255, 255, 255, 0.1)',
  color: tokens.colors.textPrimary,
  padding: '6px 14px',
} as const;

interface GeneralSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
  isSystemManagedShortcut: boolean;
  portalVersion: number;
  hotkeyBindingState: { bound: boolean; active_trigger?: string } | null;
  isApplyingHotkey: boolean;
  handleConfigureHotkey: () => void;
  checkingUpdates: boolean;
  onCheckForUpdates: () => void;
  autostartEnabled: boolean;
  onToggleAutostart: (enabled: boolean) => void;
  overlayPositioningCapabilities: { manual_offset_supported: boolean; detail?: string };
  languageOptions: { value: string; label: string }[];
}

export function GeneralSection({
  config,
  updateConfig,
  isSystemManagedShortcut,
  portalVersion,
  hotkeyBindingState,
  isApplyingHotkey,
  handleConfigureHotkey,
  checkingUpdates,
  onCheckForUpdates,
  autostartEnabled,
  onToggleAutostart,
  overlayPositioningCapabilities,
  languageOptions,
}: GeneralSectionProps) {
  const hasManuallyToggledPaste = useSignal(false);

  return (
    <>
      <ConfigField
        label="Recording Mode"
        description="Hold the hotkey while speaking, or press it to start and again to stop. Pressing the hotkey while transcribing cancels."
      >
        <ModeSwitcher
          value={config.hotkey_mode}
          onToggle={(value) => updateConfig('hotkey_mode', value)}
          options={[
            { value: 'HoldToTalk', label: 'Hold to Talk', title: 'Record while the hotkey is held down' },
            { value: 'Toggle', label: 'Press to Toggle', title: 'Press once to start recording, press again to stop' },
          ]}
        />
      </ConfigField>

      <ConfigField
        label="Global Hotkey"
        description={
          config.hotkey_mode === 'Toggle'
            ? (isSystemManagedShortcut ? 'Press your system shortcut to start recording, and again to stop.' : 'Press once to start recording, press again to stop and transcribe.')
            : (isSystemManagedShortcut ? 'Use your system shortcut to record and release to transcribe.' : 'Hold these keys to record, release to transcribe.')
        }
      >
        <div style={{ display: 'flex', gap: tokens.spacing.sm, alignItems: 'center', justifyContent: 'flex-start', width: '100%' }}>
          {!isSystemManagedShortcut && (
            <input
              type="text"
              value={config.hotkey}
              readOnly
              onClick={() => {}}
              placeholder="Configure using button"
              style={{ ...inputBaseStyle, opacity: portalVersion >= 1 ? 0.9 : 1, cursor: 'default', maxWidth: '200px' }}
              title={portalVersion >= 1 ? 'Use Configure Hotkey to request binding through the system portal.' : ''}
            />
          )}
          <Button
            size="md"
            variant="configAction"
            onClick={handleConfigureHotkey}
            disabled={isApplyingHotkey}
          >
            Modify
          </Button>
        </div>
        {!isSystemManagedShortcut && portalVersion >= 1 && (
          <div style={helperTextStyle}>
            Shortcut registration uses the Wayland GlobalShortcuts portal.
            {hotkeyBindingState?.active_trigger ? ` Active shortcut: ${hotkeyBindingState.active_trigger}.` : ''}
            {hotkeyBindingState?.bound ? ' Listener is active.' : ''}
          </div>
        )}
      </ConfigField>

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
            { value: 'Typewriter', label: 'Typewriter', title: 'Type directly into your active cursor' },
            { value: 'Clipboard', label: 'Clipboard', title: 'Copy transcription results to your clipboard' },
          ]}
        />
      </ConfigField>

      {config.output_method === 'Clipboard' && (
        <>
          <ConfigField label="Paste After Copy" description="Automatically paste after copying. Saves and restores your clipboard content.">
            <Switch name="Paste After Copy" checked={config.paste_after_copy} onChange={(checked) => {
              hasManuallyToggledPaste.value = true;
              updateConfig('paste_after_copy', checked);
            }} />
          </ConfigField>

          {config.paste_after_copy && (
            <ConfigField label="Paste Shortcut" description="Keyboard shortcut simulated to paste your transcription.">
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

      <ConfigField label="Always Copy to Clipboard" description="Also copy transcriptions to clipboard even while using Typewriter output.">
        <Switch name="Always Copy to Clipboard" checked={config.copy_on_typewriter} onChange={(checked) => updateConfig('copy_on_typewriter', checked)} />
      </ConfigField>

      <ConfigField label="Updates" description="Check for newer Voquill releases.">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-start', gap: tokens.spacing.sm, flexWrap: 'wrap', width: '100%' }}>
          <Button variant="ghost" pill style={configGhostPillStyle} onClick={onCheckForUpdates} disabled={checkingUpdates}>
            {checkingUpdates ? 'Checking...' : 'Check for Updates'}
          </Button>
        </div>
      </ConfigField>

      <ConfigField label="Launch on System Startup" description="Automatically starts Voquill when you log in.">
        <Switch name="Launch on System Startup" checked={autostartEnabled} onChange={onToggleAutostart} />
      </ConfigField>

      <ConfigField label="Status Overlay Position (px)" description="Vertical offset for the status overlay from the bottom of the screen.">
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
          <NumberField
            value={config.pixels_from_bottom}
            onChange={(value) => updateConfig('pixels_from_bottom', value)}
            min={0}
            disabled={!overlayPositioningCapabilities.manual_offset_supported}
          />
          {!overlayPositioningCapabilities.manual_offset_supported && (
            <div style={helperTextStyle}>
              {overlayPositioningCapabilities.detail || 'Manual overlay position adjustment is not available on your system.'}
            </div>
          )}
        </div>
      </ConfigField>

      <ConfigField label="Language Hint" labelBadge="Experimental" description="Best-effort language hint for transcription. Some engines/models may ignore this setting or apply it inconsistently.">
        <div style={selectWrapperStyle}>
          <SelectField
            value={config.language}
            options={languageOptions}
            onChange={(nextLanguage) => updateConfig('language', nextLanguage)}
            ariaLabel="Language hint"
          />
        </div>
      </ConfigField>
    </>
  );
}
