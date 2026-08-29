import { Button } from './Button.tsx';
import { Modal } from './Modal.tsx';
import { ModelInfoModal } from './ModelInfoModal.tsx';
import { PostProcessModelInfoModal } from './PostProcessModelInfoModal.tsx';
import { helperTextStyle, modalShortcutNoteStyle, modalShortcutPathStyle, modalTextIntroStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import type { HotkeyBindingState, SystemShortcutContext, UpdateCheckResult } from '../types.ts';

interface ModalsProps {
  showHotkeyCaptureModal: boolean;
  showSystemShortcutModal: boolean;
  showFactoryResetModal: boolean;
  showUpdateModal: boolean;
  showModelGuide: boolean;
  showPostProcessGuide: boolean;
  isRecordingHotkey: boolean;
  isApplyingHotkey: boolean;
  configHotkey: string;
  systemShortcutContext: SystemShortcutContext | null;
  hotkeyBindingState: HotkeyBindingState | null;
  updateResult: UpdateCheckResult | null;
  appVersion: string;
  isInstallingUpdate?: boolean;
  getLastCheckedLabel: () => string;
  onCancelHotkeyCapture: () => void;
  onCloseSystemShortcut: () => void;
  onChangedSystemShortcut: () => void;
  onCloseFactoryReset: () => void;
  onFactoryReset: () => void;
  onCloseUpdate: () => void;
  onInstallUpdate?: () => void;
  onOpenLatestRelease: () => void;
  onCloseModelGuide: () => void;
  onClosePostProcessGuide: () => void;
}

export function Modals(props: ModalsProps) {
  return (
    <>
      {props.showHotkeyCaptureModal && (
        <Modal
          title="Configure Hotkey"
          onClose={props.onCancelHotkeyCapture}
          maxWidth="480px"
          centerContent
          footerAlign="center"
          footer={
            <Button variant="ghost" pill onClick={props.onCancelHotkeyCapture} disabled={props.isApplyingHotkey}>
              Cancel
            </Button>
          }
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', textAlign: 'center' }}>
            <p style={{ ...helperTextStyle, fontSize: tokens.typography.sizeSm }}>
              Press your desired key combination on your keyboard, or press Escape to cancel.
            </p>
            <div
              style={{
                border: '1px solid rgba(88, 101, 242, 0.4)',
                background: 'rgba(88, 101, 242, 0.1)',
                borderRadius: '10px',
                padding: '16px 20px',
                fontSize: tokens.typography.sizeMd,
                fontWeight: 700,
                color: props.isRecordingHotkey ? '#c7d2fe' : tokens.colors.textPrimary,
                boxShadow: '0 4px 16px rgba(0, 0, 0, 0.25)',
              }}
            >
              {props.isRecordingHotkey ? 'Listening for key combination...' : props.configHotkey}
            </div>
          </div>
        </Modal>
      )}

      {props.showSystemShortcutModal && (
        <Modal
          title="Change Shortcut"
          onClose={props.onCloseSystemShortcut}
          maxWidth="560px"
          centerContent
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
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <p style={{ ...modalTextIntroStyle, fontSize: tokens.typography.sizeMd }}>
              {props.systemShortcutContext?.desktop
                ? `Your ${props.systemShortcutContext.desktop} desktop manages this shortcut${props.systemShortcutContext?.distro ? ` on ${props.systemShortcutContext.distro}` : ''}. To change it, open:`
                : props.systemShortcutContext?.distro
                  ? `Your ${props.systemShortcutContext.distro} system manages this shortcut. To change it, open:`
                  : 'Your system manages this shortcut. To change it, open:'}
            </p>
            <div
              style={{
                ...modalShortcutPathStyle,
                background: 'rgba(255, 255, 255, 0.05)',
                border: '1px solid rgba(255, 255, 255, 0.1)',
                borderRadius: '8px',
                padding: '10px 14px',
              }}
            >
              {props.systemShortcutContext?.settings_path || 'Settings -> Apps -> Voquill -> Global Shortcuts'}
            </div>
            {props.hotkeyBindingState?.active_trigger && (
              <p style={modalShortcutNoteStyle}>
                Current shortcut: <strong>{props.hotkeyBindingState.active_trigger}</strong>
              </p>
            )}
            <p style={modalShortcutNoteStyle}>
              If you can&apos;t find it, you may need to search through your system settings for &quot;Voquill&quot; or &quot;shortcuts&quot;.
            </p>
          </div>
        </Modal>
      )}

      {props.showFactoryResetModal && (
        <Modal
          title="Factory Reset"
          onClose={props.onCloseFactoryReset}
          maxWidth="520px"
          centerContent
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
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', textAlign: 'center' }}>
            <p style={{ ...modalTextIntroStyle, fontSize: tokens.typography.sizeMd, margin: 0 }}>
              This will reset Voquill to defaults and permanently clear downloaded models, logs, and history.
            </p>
            <p style={{ ...modalShortcutNoteStyle, color: '#f87171', fontWeight: 600, margin: 0 }}>
              This action cannot be undone.
            </p>
          </div>
        </Modal>
      )}

      {props.showUpdateModal && (
        <Modal
          title={props.updateResult?.updateAvailable ? 'Update Available' : 'Voquill is Up to Date'}
          onClose={props.onCloseUpdate}
          maxWidth="540px"
          centerContent
          footerAlign="center"
          footer={
            <>
              <Button variant="ghost" pill onClick={props.onCloseUpdate} disabled={props.isInstallingUpdate}>
                {props.updateResult?.updateAvailable ? 'Later' : 'Close'}
              </Button>
              {props.updateResult?.updateAvailable && (
                <>
                  <Button variant="ghost" pill onClick={props.onOpenLatestRelease} disabled={props.isInstallingUpdate}>
                    Release Notes
                  </Button>
                  {props.onInstallUpdate && (
                    <Button variant="primary" pill onClick={props.onInstallUpdate} disabled={props.isInstallingUpdate}>
                      {props.isInstallingUpdate ? 'Updating...' : 'Update Now'}
                    </Button>
                  )}
                </>
              )}
            </>
          }
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <p style={{ ...modalTextIntroStyle, fontSize: tokens.typography.sizeMd, margin: 0 }}>
              {props.updateResult?.updateAvailable
                ? `A newer Voquill version is available. Current: v${props.updateResult.currentVersion} -> Latest: v${props.updateResult.latestVersion}.`
                : `You are on the latest version (v${props.updateResult?.currentVersion || props.appVersion}).`}
            </p>
            {props.updateResult?.updateAvailable && (
              <p style={{ ...modalShortcutNoteStyle, margin: 0 }}>
                {props.isInstallingUpdate
                  ? 'Downloading and applying the update. Voquill will close and relaunch automatically once completed.'
                  : 'Click "Update Now" to automatically download and install the latest release. Voquill will restart after updating.'}
              </p>
            )}
            <p style={{ ...modalShortcutNoteStyle, margin: 0 }}>
              Last checked: {props.getLastCheckedLabel()}
            </p>
          </div>
        </Modal>
      )}

      {props.showModelGuide && <ModelInfoModal onClose={props.onCloseModelGuide} />}
      {props.showPostProcessGuide && (
        <PostProcessModelInfoModal onClose={props.onClosePostProcessGuide} />
      )}
    </>
  );
}