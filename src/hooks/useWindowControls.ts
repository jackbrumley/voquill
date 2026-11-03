import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface UseWindowControlsReturn {
  isMaximized: boolean;
  handleClose: () => Promise<void>;
  handleMinimize: () => Promise<void>;
  toggleWindowMaximize: () => Promise<void>;
  handleTitleBarMouseDown: (event: MouseEvent) => Promise<void>;
  handleTitleBarDoubleClick: (event: MouseEvent) => Promise<void>;
  handleResizeCornerMouseDown: (direction: ResizeDirection) => (event: MouseEvent) => Promise<void>;
}

/// Where the backend parked the main window (`tray::WindowParking`).
type WindowParking = 'tray' | 'taskbar';

type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West';

export function useWindowControls(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseWindowControlsReturn {
  const trayFallbackNotified = useSignal(false);
  const isMaximized = useSignal(false);

  useEffect(() => {
    const appWindow = getCurrentWindow();

    const syncMaximizeState = async () => {
      try {
        isMaximized.value = await appWindow.isMaximized();
      } catch (err) {
        console.error('Failed to read maximize state:', err);
      }
    };

    void syncMaximizeState();
    const unlistenPromise = appWindow.onResized(() => {
      void syncMaximizeState();
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch((err) => {
        console.error('Failed to stop listening for window resizes:', err);
      });
    };
  }, []);

  const notifyIfParkedInTaskbar = (parking: WindowParking | null) => {
    if (parking === 'taskbar' && !trayFallbackNotified.value) {
      trayFallbackNotified.value = true;
      showToast('System tray is unavailable on this desktop. Minimized to taskbar instead.', 'info');
    }
  };

  const handleClose = async () => {
    try {
      notifyIfParkedInTaskbar(await invoke<WindowParking | null>('close_main_window'));
    } catch {
      await getCurrentWindow().close();
    }
  };

  const handleMinimize = async () => {
    try {
      notifyIfParkedInTaskbar(await invoke<WindowParking>('minimize_to_tray_or_taskbar'));
    } catch {
      await getCurrentWindow().minimize();
    }
  };

  const toggleWindowMaximize = async () => {
    try {
      const win = getCurrentWindow();
      if (await win.isMaximized()) {
        await win.unmaximize();
      } else {
        await win.maximize();
      }
      isMaximized.value = await win.isMaximized();
    } catch {
      // no-op if maximize is unavailable
    }
  };

  const handleTitleBarMouseDown = async (event: MouseEvent) => {
    const target = event.target as HTMLElement | null;
    if (event.detail > 1) {
      event.preventDefault();
      return;
    }

    if (event.buttons === 1 && !target?.closest('button')) {
      event.preventDefault();
      await getCurrentWindow().startDragging();
    }
  };

  const handleTitleBarDoubleClick = async (event: MouseEvent) => {
    const target = event.target as HTMLElement | null;
    if (target?.closest('button')) {
      return;
    }

    event.preventDefault();
    await toggleWindowMaximize();
  };

  const handleResizeCornerMouseDown = (direction: ResizeDirection) => async (event: MouseEvent) => {
    if (event.buttons !== 1 || isMaximized.value) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    try {
      await getCurrentWindow().startResizeDragging(direction);
    } catch (err) {
      console.error('Failed to start resize dragging:', err);
    }
  };

  return { isMaximized: isMaximized.value, handleClose, handleMinimize, toggleWindowMaximize, handleTitleBarMouseDown, handleTitleBarDoubleClick, handleResizeCornerMouseDown };
}