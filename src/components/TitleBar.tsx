import { IconMinus, IconSquare, IconX } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { titleBarStyle, titleBarTitleStyle, titleBarControlsStyle } from '../theme/ui-primitives.ts';

interface TitleBarProps {
  onMinimize: () => void;
  onMaximize: () => void;
  onClose: () => void;
  onMouseDown: (event: MouseEvent) => void;
  onDoubleClick: (event: MouseEvent) => void;
}

export function TitleBar({ onMinimize, onMaximize, onClose, onMouseDown, onDoubleClick }: TitleBarProps) {
  return (
    <div
      data-tauri-drag-region
      style={titleBarStyle}
      onMouseDown={onMouseDown}
      onDblClick={onDoubleClick}
    >
      <div style={titleBarTitleStyle}>Voquill</div>
      <div style={titleBarControlsStyle}>
        <Button variant="icon" onClick={onMinimize}>
          <IconMinus size={16} />
        </Button>
        <Button variant="icon" onClick={onMaximize}>
          <IconSquare size={14} />
        </Button>
        <Button variant="icon" onClick={onClose}>
          <IconX size={16} />
        </Button>
      </div>
    </div>
  );
}