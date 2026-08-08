import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import type { HotkeyBindingState, SystemShortcutContext, OverlayPositioningCapabilities, ConfigureHotkeyResult } from '../types.ts';

interface UseHotkeySetupOptions {
  showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
  onApplyCapturedHotkey: (normalized: string) => Promise<void>;
}

interface UseHotkeySetupReturn {
  hotkeyBindingState: HotkeyBindingState | null;
  systemShortcutContext: SystemShortcutContext | null;
  overlayPositioningCapabilities: OverlayPositioningCapabilities;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  isRecordingHotkey: boolean;
  isApplyingHotkey: boolean;
  showHotkeyCaptureModal: boolean;
  showSystemShortcutModal: boolean;
  showFactoryResetModal: boolean;
  setHotkeyBindingState: (state: HotkeyBindingState | null) => void;
  setSystemShortcutContext: (context: SystemShortcutContext | null) => void;
  setShowHotkeyCaptureModal: (show: boolean) => void;
  setShowSystemShortcutModal: (show: boolean) => void;
  setShowFactoryResetModal: (show: boolean) => void;
  setIsApplyingHotkey: (applying: boolean) => void;
  handleConfigureHotkey: () => Promise<void>;
  setRecordingState: (isRecording: boolean) => Promise<void>;
  cancelHotkeyCapture: () => Promise<void>;
  handleHotkeyKeyDown: (e: KeyboardEvent) => void;
  handleHotkeyKeyUp: (e: KeyboardEvent) => void;
}

export function useHotkeySetup(options: UseHotkeySetupOptions): UseHotkeySetupReturn {
  const { showToast, onApplyCapturedHotkey } = options;
  const hotkeyBindingState = useSignal<HotkeyBindingState | null>(null);
  const systemShortcutContext = useSignal<SystemShortcutContext | null>(null);
  const overlayPositioningCapabilities = useSignal<OverlayPositioningCapabilities>({
    manual_offset_supported: false,
    detail: 'Manual overlay position adjustment is not available on your system.',
  });
  const portalVersion = useSignal<number>(0);
  const isRecordingHotkey = useSignal(false);
  const recordedKeys = useSignal<Set<string>>(new Set());
  const isApplyingHotkey = useSignal(false);
  const showHotkeyCaptureModal = useSignal(false);
  const showSystemShortcutModal = useSignal(false);
  const showFactoryResetModal = useSignal(false);

  const isSystemManagedShortcut = portalVersion.value >= 1;

  useEffect(() => {
    invoke<number>('get_wayland_portal_version')
      .then((v) => { portalVersion.value = v; })
      .catch(e => console.log("Not running Wayland portal version check:", e));

    invoke<HotkeyBindingState>('get_hotkey_binding_state')
      .then((v) => { hotkeyBindingState.value = v; })
      .catch(e => console.log('Hotkey binding state unavailable:', e));

    invoke<SystemShortcutContext>('get_system_shortcut_context')
      .then((v) => { systemShortcutContext.value = v; })
      .catch(e => console.log('System shortcut context unavailable:', e));

    invoke<OverlayPositioningCapabilities>('get_overlay_positioning_capabilities')
      .then((v) => { overlayPositioningCapabilities.value = v; })
      .catch(e => {
        overlayPositioningCapabilities.value = {
          manual_offset_supported: false,
          detail: 'Manual overlay position adjustment is not available on your system.',
        };
        console.log('Overlay positioning capabilities unavailable:', e);
      });
  }, []);

  const handleConfigureHotkey = async () => {
    if (isApplyingHotkey.value) return;

    try {
      isApplyingHotkey.value = true;
      const result = await invoke<ConfigureHotkeyResult>('configure_hotkey');

      if (result.outcome === 'requires_in_app_capture') {
        showHotkeyCaptureModal.value = true;
        isRecordingHotkey.value = true;
        recordedKeys.value = new Set();
        showToast('Press your desired key combination in the modal.', 'info');
      } else if (result.outcome === 'system_managed') {
        showSystemShortcutModal.value = true;
      } else {
        showToast(result.detail || 'Shortcut configured successfully!', 'success');
      }
    } catch (error) {
      showToast(`Failed to configure shortcut: ${error}`, 'error');
    } finally {
      isApplyingHotkey.value = false;
    }
  };

  const normalizeHotkey = (keys: Set<string>): string => {
    const modifiers: string[] = [];
    let primaryKey = '';

    keys.forEach(key => {
      const lower = key.toLowerCase();
      if (lower === 'control' || lower === 'controlleft' || lower === 'controlright') modifiers.push('Ctrl');
      else if (lower === 'shift' || lower === 'shiftleft' || lower === 'shiftright') modifiers.push('Shift');
      else if (lower === 'alt' || lower === 'altleft' || lower === 'altright') modifiers.push('Alt');
      else if (lower === 'meta' || lower === 'metaleft' || lower === 'metaright' || lower === 'osleft' || lower === 'osright') modifiers.push('Super');
      else if (key.startsWith('Key')) {
        primaryKey = key.slice(3);
      } else if (key === 'Space') {
        primaryKey = 'Space';
      } else {
        primaryKey = key.charAt(0).toUpperCase() + key.slice(1).toLowerCase();
      }
    });

    return [...modifiers.sort(), primaryKey].filter(Boolean).join('+');
  };

  const setRecordingState = async (isRecording: boolean) => {
    isRecordingHotkey.value = isRecording;
    try {
      await invoke('set_configuring_hotkey', { isConfiguring: isRecording });
    } catch (e) {
      console.error('Failed to sync configuring hotkey state', e);
    }
  };

  const cancelHotkeyCapture = async () => {
    await setRecordingState(false);
    recordedKeys.value = new Set();
    showHotkeyCaptureModal.value = false;
    showToast('Hotkey configuration cancelled.', 'info');
  };

  const handleHotkeyKeyDown = (e: KeyboardEvent) => {
    if (!isRecordingHotkey.value) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.repeat) return;

    if (e.key === 'Escape') {
      void cancelHotkeyCapture();
      return;
    }

    const newKeys = new Set(recordedKeys.value);
    if (e.ctrlKey) newKeys.add('Control');
    if (e.shiftKey) newKeys.add('Shift');
    if (e.altKey) newKeys.add('Alt');
    if (e.metaKey) newKeys.add('Meta');

    const code = e.code;
    const modifierCodes = [
      'ControlLeft', 'ControlRight', 'ShiftLeft', 'ShiftRight',
      'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight',
    ];

    if (!modifierCodes.includes(code)) {
      newKeys.add(code);
      const normalized = normalizeHotkey(newKeys).toLowerCase();
      if (!normalized || ['ctrl', 'shift', 'alt', 'super'].includes(normalized)) {
        showToast('Please include a non-modifier key in the shortcut.', 'error');
        recordedKeys.value = newKeys;
        return;
      }
      void onApplyCapturedHotkey(normalized);
    } else {
      recordedKeys.value = newKeys;
    }
  };

  const handleHotkeyKeyUp = (e: KeyboardEvent) => {
    if (!isRecordingHotkey.value) return;
    e.preventDefault();
    e.stopPropagation();
  };

  return {
    hotkeyBindingState: hotkeyBindingState.value,
    systemShortcutContext: systemShortcutContext.value,
    overlayPositioningCapabilities: overlayPositioningCapabilities.value,
    portalVersion: portalVersion.value,
    isSystemManagedShortcut,
    isRecordingHotkey: isRecordingHotkey.value,
    isApplyingHotkey: isApplyingHotkey.value,
    showHotkeyCaptureModal: showHotkeyCaptureModal.value,
    showSystemShortcutModal: showSystemShortcutModal.value,
    showFactoryResetModal: showFactoryResetModal.value,
    setHotkeyBindingState: (state) => { hotkeyBindingState.value = state; },
    setSystemShortcutContext: (context) => { systemShortcutContext.value = context; },
    setShowHotkeyCaptureModal: (show) => { showHotkeyCaptureModal.value = show; },
    setShowSystemShortcutModal: (show) => { showSystemShortcutModal.value = show; },
    setShowFactoryResetModal: (show) => { showFactoryResetModal.value = show; },
    setIsApplyingHotkey: (applying) => { isApplyingHotkey.value = applying; },
    handleConfigureHotkey,
    setRecordingState,
    cancelHotkeyCapture,
    handleHotkeyKeyDown,
    handleHotkeyKeyUp,
  };
}