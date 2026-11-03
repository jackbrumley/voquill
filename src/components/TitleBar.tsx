import { IconCopy, IconMinus, IconSquare, IconX } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { titleBarStyle, titleBarTitleStyle, titleBarControlsStyle } from '../theme/ui-primitives.ts';

interface TitleBarProps {
  isMaximized: boolean;
  onMinimize: () => void;
  onMaximize: () => void;
  onClose: () => void;
  onMouseDown: (event: MouseEvent) => void;
  onDoubleClick: (event: MouseEvent) => void;
}

export function TitleBar({ isMaximized, onMinimize, onMaximize, onClose, onMouseDown, onDoubleClick }: TitleBarProps) {
  return (
    <div
      data-tauri-drag-region
      style={titleBarStyle}
      onMouseDown={onMouseDown}
      onDblClick={onDoubleClick}
    >
      <div style={titleBarTitleStyle}>
        <img src="/logo.svg" alt="Voquill" style={{ height: 18, width: 18, marginRight: 7 }} />
        Voquill
      </div>
      <div style={titleBarControlsStyle}>
        <Button variant="titlebarIcon" onClick={onMinimize} ariaLabel="Minimize window">
          <IconMinus size={14} />
        </Button>
        <Button
          variant="titlebarIcon"
          onClick={onMaximize}
          ariaLabel={isMaximized ? 'Restore window' : 'Maximize window'}
        >
          {isMaximized ? <IconCopy size={12} /> : <IconSquare size={12} />}
        </Button>
        <Button variant="titlebarClose" onClick={onClose} ariaLabel="Close window">
          <IconX size={14} />
        </Button>
      </div>
    </div>
  );
}