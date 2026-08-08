import { useState, useCallback, useEffect } from 'preact/hooks';
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
  recordedKeys: Set<string>;
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
  const [hotkeyBindingState, setHotkeyBindingState] = useState<HotkeyBindingState | null>(null);
  const [systemShortcutContext, setSystemShortcutContext] = useState<SystemShortcutContext | null>(null);
  const [overlayPositioningCapabilities, setOverlayPositioningCapabilities] = useState<OverlayPositioningCapabilities>({
    manual_offset_supported: false,
    detail: 'Manual overlay position adjustment is not available on your system.',
  });
  const [portalVersion, setPortalVersion] = useState<number>(0);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [recordedKeys, setRecordedKeys] = useState<Set<string>>(new Set());
  const [isApplyingHotkey, setIsApplyingHotkey] = useState(false);
  const [showHotkeyCaptureModal, setShowHotkeyCaptureModal] = useState(false);
  const [showSystemShortcutModal, setShowSystemShortcutModal] = useState(false);
  const [showFactoryResetModal, setShowFactoryResetModal] = useState(false);

  const isSystemManagedShortcut = portalVersion >= 1;

  useEffect(() => {
    invoke<number>('get_wayland_portal_version')
      .then(setPortalVersion)
      .catch(e => console.log("Not running Wayland portal version check:", e));

    invoke<HotkeyBindingState>('get_hotkey_binding_state')
      .then(setHotkeyBindingState)
      .catch(e => console.log('Hotkey binding state unavailable:', e));

    invoke<SystemShortcutContext>('get_system_shortcut_context')
      .then(setSystemShortcutContext)
      .catch(e => console.log('System shortcut context unavailable:', e));

    invoke<OverlayPositioningCapabilities>('get_overlay_positioning_capabilities')
      .then(setOverlayPositioningCapabilities)
      .catch(e => {
        setOverlayPositioningCapabilities({
          manual_offset_supported: false,
          detail: 'Manual overlay position adjustment is not available on your system.',
        });
        console.log('Overlay positioning capabilities unavailable:', e);
      });
  }, []);

  const handleConfigureHotkey = useCallback(async () => {
    if (isApplyingHotkey) return;

    try {
      setIsApplyingHotkey(true);
      const result = await invoke<ConfigureHotkeyResult>('configure_hotkey');

      if (result.outcome === 'requires_in_app_capture') {
        setShowHotkeyCaptureModal(true);
        setIsRecordingHotkey(true);
        setRecordedKeys(new Set());
        showToast('Press your desired key combination in the modal.', 'info');
      } else if (result.outcome === 'system_managed') {
        setShowSystemShortcutModal(true);
      } else {
        showToast(result.detail || 'Shortcut configured successfully!', 'success');
      }
    } catch (error) {
      showToast(`Failed to configure shortcut: ${error}`, 'error');
    } finally {
      setIsApplyingHotkey(false);
    }
  }, [isApplyingHotkey, showToast]);

  const normalizeHotkey = useCallback((keys: Set<string>): string => {
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
  }, []);

  const setRecordingState = useCallback(async (isRecording: boolean) => {
    setIsRecordingHotkey(isRecording);
    try {
      await invoke('set_configuring_hotkey', { isConfiguring: isRecording });
    } catch (e) {
      console.error('Failed to sync configuring hotkey state', e);
    }
  }, []);

  const cancelHotkeyCapture = useCallback(async () => {
    await setRecordingState(false);
    setRecordedKeys(new Set());
    setShowHotkeyCaptureModal(false);
    showToast('Hotkey configuration cancelled.', 'info');
  }, [showToast, setRecordingState]);

  const handleHotkeyKeyDown = useCallback((e: KeyboardEvent) => {
    if (!isRecordingHotkey) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.repeat) return;

    if (e.key === 'Escape') {
      void cancelHotkeyCapture();
      return;
    }

    const newKeys = new Set(recordedKeys);
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
        setRecordedKeys(newKeys);
        return;
      }
      void onApplyCapturedHotkey(normalized);
    } else {
      setRecordedKeys(newKeys);
    }
  }, [isRecordingHotkey, recordedKeys, cancelHotkeyCapture, normalizeHotkey, showToast, onApplyCapturedHotkey]);

  const handleHotkeyKeyUp = useCallback((e: KeyboardEvent) => {
    if (!isRecordingHotkey) return;
    e.preventDefault();
    e.stopPropagation();
  }, [isRecordingHotkey]);

  return {
    hotkeyBindingState,
    systemShortcutContext,
    overlayPositioningCapabilities,
    portalVersion,
    isSystemManagedShortcut,
    isRecordingHotkey,
    recordedKeys,
    isApplyingHotkey,
    showHotkeyCaptureModal,
    showSystemShortcutModal,
    showFactoryResetModal,
    setHotkeyBindingState,
    setSystemShortcutContext,
    setShowHotkeyCaptureModal,
    setShowSystemShortcutModal,
    setShowFactoryResetModal,
    setIsApplyingHotkey,
    handleConfigureHotkey,
    setRecordingState,
    cancelHotkeyCapture,
    handleHotkeyKeyDown,
    handleHotkeyKeyUp,
  };
}