import { Button } from './Button.tsx';
import { Modal } from './Modal.tsx';
import { ModelInfoModal } from './ModelInfoModal.tsx';
import { helperTextStyle, modalShortcutNoteStyle, modalShortcutPathStyle, modalTextIntroStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import type { HotkeyBindingState, SystemShortcutContext, UpdateCheckResult } from '../types.ts';

interface ModalsProps {
  showHotkeyCaptureModal: boolean;
  showSystemShortcutModal: boolean;
  showFactoryResetModal: boolean;
  showUpdateModal: boolean;
  showModelGuide: boolean;
  isRecordingHotkey: boolean;
  isApplyingHotkey: boolean;
  configHotkey: string;
  systemShortcutContext: SystemShortcutContext | null;
  hotkeyBindingState: HotkeyBindingState | null;
  updateResult: UpdateCheckResult | null;
  appVersion: string;
  getLastCheckedLabel: () => string;
  onCancelHotkeyCapture: () => void;
  onCloseSystemShortcut: () => void;
  onChangedSystemShortcut: () => void;
  onCloseFactoryReset: () => void;
  onFactoryReset: () => void;
  onCloseUpdate: () => void;
  onOpenLatestRelease: () => void;
  onCloseModelGuide: () => void;
}

export function Modals(props: ModalsProps) {
  return (
    <>
      {props.showHotkeyCaptureModal && (
        <Modal
          title="Configure Hotkey"
          onClose={props.onCancelHotkeyCapture}
          maxWidth="440px"
          footerAlign="center"
          footer={
            <Button variant="ghost" pill onClick={props.onCancelHotkeyCapture} disabled={props.isApplyingHotkey}>
              Cancel
            </Button>
          }
        >
          <p style={helperTextStyle}>
            Press your desired key combination, or press Escape to cancel.
          </p>
          <div style={{ border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', padding: '10px 12px', textAlign: 'center', fontWeight: 700 }}>
            {props.isRecordingHotkey ? 'Listening for keys...' : props.configHotkey}
          </div>
        </Modal>
      )}

      {props.showSystemShortcutModal && (
        <Modal
          title="Change Shortcut"
          onClose={props.onCloseSystemShortcut}
          maxWidth="560px"
          footerAlign="center"
          footer={
            <>
              <Button variant="ghost" pill onClick={props.onCloseSystemShortcut}>
                Close
              </Button>
              <Button variant="primary" pill onClick={props.onChangedSystemShortcut}>
                I changed it
              </Button>
            </>
          }
        >
          <p style={{ ...modalTextIntroStyle, fontSize: tokens.typography.sizeMd }}>
            {props.systemShortcutContext?.desktop
              ? `Your ${props.systemShortcutContext.desktop} desktop manages this shortcut${props.systemShortcutContext?.distro ? ` on ${props.systemShortcutContext.distro}` : ''}. To change it, open:`
              : props.systemShortcutContext?.distro
                ? `Your ${props.systemShortcutContext.distro} system manages this shortcut. To change it, open:`
                : 'Your system manages this shortcut. To change it, open:'}
          </p>
          <p style={modalShortcutPathStyle}>
            {props.systemShortcutContext?.settings_path || 'Settings -> Apps -> Voquill -> Global Shortcuts'}
          </p>
          {props.hotkeyBindingState?.active_trigger && (
            <p style={modalShortcutNoteStyle}>
              Current shortcut: {props.hotkeyBindingState.active_trigger}
            </p>
          )}
          <p style={modalShortcutNoteStyle}>
            If you can&apos;t find it, you may need to search through your system settings for &quot;Voquill&quot; or &quot;shortcuts&quot;.
          </p>
        </Modal>
      )}

      {props.showFactoryResetModal && (
        <Modal
          title="Factory Reset"
          onClose={props.onCloseFactoryReset}
          maxWidth="560px"
          footerAlign="center"
          footer={
            <>
              <Button variant="ghost" pill onClick={props.onCloseFactoryReset}>
                Cancel
              </Button>
              <Button variant="danger" pill onClick={props.onFactoryReset}>
                Reset Everything
              </Button>
            </>
          }
        >
          <p style={modalTextIntroStyle}>
            This will reset Voquill to defaults and permanently clear downloaded models, logs, and history.
          </p>
          <p style={modalShortcutNoteStyle}>This action cannot be undone.</p>
        </Modal>
      )}

      {props.showUpdateModal && (
        <Modal
          title={props.updateResult?.updateAvailable ? 'Update Available' : 'Voquill is Up to Date'}
          onClose={props.onCloseUpdate}
          maxWidth="560px"
          footerAlign="center"
          footer={
            <>
              <Button variant="ghost" pill onClick={props.onCloseUpdate}>
                Later
              </Button>
              <Button variant="primary" pill onClick={props.onOpenLatestRelease}>
                Download Latest
              </Button>
            </>
          }
        >
          <p style={modalTextIntroStyle}>
            {props.updateResult?.updateAvailable
              ? `A newer Voquill version is available. Current: v${props.updateResult.currentVersion} -> Latest: v${props.updateResult.latestVersion}.`
              : `You are on the latest version (v${props.updateResult?.currentVersion || props.appVersion}).`}
          </p>
          <p style={modalShortcutNoteStyle}>
            Updates are currently installed manually by downloading the latest release package.
          </p>
          <p style={modalShortcutNoteStyle}>Last checked: {props.getLastCheckedLabel()}</p>
        </Modal>
      )}

      {props.showModelGuide && <ModelInfoModal onClose={props.onCloseModelGuide} />}
    </>
  );
}