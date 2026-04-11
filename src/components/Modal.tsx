import { ComponentChildren } from 'preact';
import { useEffect } from 'preact/hooks';
import { IconX } from '@tabler/icons-preact';
import { Card } from './Card.tsx';
import { Button } from './Button.tsx';
import { tokens } from '../design-tokens.ts';

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ComponentChildren;
  footer?: ComponentChildren;
  maxWidth?: string;
  closeOnOverlay?: boolean;
  fullScreen?: boolean;
  hideCloseButton?: boolean;
  footerAlign?: 'end' | 'center';
}

export function Modal({
  title,
  onClose,
  children,
  footer,
  maxWidth = '500px',
  closeOnOverlay = true,
  fullScreen = false,
  hideCloseButton = false,
  footerAlign = 'end',
}: ModalProps) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  return (
    <div
      onClick={closeOnOverlay ? onClose : undefined}
      style={{
        position: 'fixed',
        top: fullScreen ? '42px' : 0,
        right: 0,
        bottom: 0,
        left: 0,
        zIndex: 1000,
        background: fullScreen
          ? `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`
          : 'rgba(0, 0, 0, 0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: fullScreen ? '0' : '16px',
      }}
    >
      <div
        onClick={(event: MouseEvent) => event.stopPropagation()}
        style={{
          width: '100%',
          maxWidth: fullScreen ? 'none' : maxWidth,
          height: fullScreen ? '100%' : 'auto',
        }}
      >
        <Card
          className="modal-card"
          style={fullScreen
            ? {
                height: '100%',
                display: 'flex',
                flexDirection: 'column',
                background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
                backdropFilter: 'none',
                WebkitBackdropFilter: 'none',
                boxShadow: 'none',
                overflowY: 'auto',
                scrollbarGutter: 'stable',
              }
            : undefined}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: hideCloseButton ? 'center' : 'space-between', marginBottom: '12px' }}>
            <h2 style={{ fontSize: '18px', margin: 0, color: tokens.colors.textPrimary }}>{title}</h2>
            {!hideCloseButton && (
              <Button variant="icon" onClick={onClose} title="Close" style={{ width: '34px', height: '34px' }}>
                <IconX size={20} />
              </Button>
            )}
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', flex: fullScreen ? 1 : undefined, minHeight: fullScreen ? 0 : undefined }}>{children}</div>

          {footer && (
            <div style={{ display: 'flex', justifyContent: footerAlign === 'center' ? 'center' : 'flex-end', gap: '8px', marginTop: '16px' }}>
              {footer}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
