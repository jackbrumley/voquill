import { ComponentChildren } from 'preact';
import { IconCheck } from '@tabler/icons-preact';
import { Button } from '../../components/Button.tsx';
import { helperTextStyle, inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import type { PlatformPermissions, SetupConfig } from './setupTypes.ts';

interface PermissionsStepProps {
  permissions: PlatformPermissions | null;
  config: SetupConfig;
  isHotkeyReady: boolean;
  hotkeyError: string | null;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  isApplyingHotkey: boolean;
  isRecordingHotkey: boolean;
  onAudioSetup: () => void;
  onInputSetup: () => void;
  onConfigureHotkey: () => void;
  onHotkeyKeyDown: (event: KeyboardEvent) => void;
  onHotkeyKeyUp: (event: KeyboardEvent) => void;
  onHotkeyBlur: () => void;
}

const warningTextStyle = { ...helperTextStyle, color: '#f1c40f' } as const;

interface PermissionRowProps {
  title: string;
  description: string;
  isGranted: boolean;
  action: ComponentChildren;
  children?: ComponentChildren;
}

function PermissionRow({ title, description, isGranted, action, children }: PermissionRowProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.sm }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textPrimary, fontWeight: 500 }}>{title}</div>
          <div style={helperTextStyle}>{description}</div>
        </div>
        {isGranted ? <IconCheck color="var(--colors-success)" size={20} /> : action}
      </div>
      {children}
    </div>
  );
}

/// Step 1: the permissions the platform gates. Backends that gate nothing
/// (X11, Windows) report everything granted, so this step finishes itself.
export function PermissionsStep(props: PermissionsStepProps) {
  const { permissions, config, isHotkeyReady, hotkeyError } = props;
  const isShortcutReady = Boolean(permissions?.shortcuts) && isHotkeyReady && !hotkeyError;

  return (
    <>
      <PermissionRow
        title="Microphone access"
        description="So Voquill can hear you."
        isGranted={Boolean(permissions?.audio)}
        action={<Button variant="secondary" size="sm" pill onClick={props.onAudioSetup}>Allow</Button>}
      />

      <PermissionRow
        title="Keyboard shortcut"
        description={`So ${config.hotkey} starts dictation from any app.`}
        isGranted={isShortcutReady}
        action={
          <Button variant="secondary" size="sm" pill onClick={props.onConfigureHotkey} disabled={props.isApplyingHotkey}>
            Set up
          </Button>
        }
      >
        {!permissions?.shortcuts && !props.isSystemManagedShortcut && (
          <input
            type="text"
            value={props.isRecordingHotkey ? 'Press keys...' : config.hotkey}
            onKeyDown={props.onHotkeyKeyDown}
            onKeyUp={props.onHotkeyKeyUp}
            onBlur={props.onHotkeyBlur}
            readOnly
            placeholder={props.portalVersion >= 1 ? 'Bind with button' : 'Click to set'}
            className="hotkey-input"
            style={{
              ...inputBaseStyle,
              width: '100%',
              maxWidth: '240px',
              cursor: props.portalVersion >= 1 ? 'default' : 'pointer',
              color: tokens.colors.textPrimary,
              opacity: props.portalVersion >= 1 ? 0.85 : 1,
            }}
          />
        )}
        {!permissions?.shortcuts && permissions?.shortcuts_detail && (
          <div style={warningTextStyle}>{permissions.shortcuts_detail}</div>
        )}
        {hotkeyError && <div style={warningTextStyle}>{hotkeyError}</div>}
      </PermissionRow>

      <PermissionRow
        title="Typing into other apps"
        description="So Voquill can type what you say."
        isGranted={Boolean(permissions?.input_emulation)}
        action={<Button variant="secondary" size="sm" pill onClick={props.onInputSetup}>Allow</Button>}
      />
    </>
  );
}
