import { ComponentChildren } from 'preact';
import { IconX } from '@tabler/icons-preact';
import { Card } from './Card.tsx';
import { Button } from './Button.tsx';

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ComponentChildren;
  footer?: ComponentChildren;
  maxWidth?: string;
  closeOnOverlay?: boolean;
}

export function Modal({
  title,
  onClose,
  children,
  footer,
  maxWidth = '500px',
  closeOnOverlay = true,
}: ModalProps) {
  return (
    <div className="modal-overlay" onClick={closeOnOverlay ? onClose : undefined}>
      <div
        onClick={(event: MouseEvent) => event.stopPropagation()}
        style={{ width: '100%', maxWidth }}
      >
        <Card className="modal-card">
          <div className="modal-header">
            <h2>{title}</h2>
            <Button variant="icon" onClick={onClose} title="Close">
              <IconX size={20} />
            </Button>
          </div>

          <div className="modal-body">{children}</div>

          {footer && <div className="modal-footer">{footer}</div>}
        </Card>
      </div>
    </div>
  );
}
