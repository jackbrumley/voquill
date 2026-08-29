import { ComponentChildren } from 'preact';
import { useEffect } from 'preact/hooks';
import { IconX } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';
import { appBackground, titleBarHeight } from '../theme/ui-primitives.ts';

export interface ModalProps {
  title?: string;
  subtitle?: string;
  onClose: () => void;
  children: ComponentChildren;
  footer?: ComponentChildren;
  maxWidth?: string;
  centerContent?: boolean;
  footerAlign?: 'end' | 'center' | 'space-between';
  showCloseButton?: boolean;
  headerSlot?: ComponentChildren;
  fullScreen?: boolean;
  closeOnOverlay?: boolean;
}

export function Modal({
  title,
  subtitle,
  onClose,
  children,
  footer,
  maxWidth,
  centerContent = false,
  footerAlign = 'end',
  showCloseButton = true,
  headerSlot,
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

  const hasHeader = Boolean(title || subtitle || headerSlot);

  return (
    <div
      role="dialog"
      aria-modal="true"
      style={{
        position: 'fixed',
        top: titleBarHeight,
        right: 0,
        bottom: 0,
        left: 0,
        zIndex: 1000,
        background: appBackground,
        color: tokens.colors.textPrimary,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
    >
      {/* Top Header Bar */}
      {hasHeader && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '10px 16px',
            background: 'rgba(0, 0, 0, 0.2)',
            borderBottom: '1px solid rgba(255, 255, 255, 0.06)',
            flexShrink: 0,
            minHeight: '46px',
            gap: '12px',
          }}
        >
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', minWidth: 0, flex: 1 }}>
            {title && (
              <h2
                style={{
                  fontSize: '15px',
                  fontWeight: 700,
                  margin: 0,
                  color: tokens.colors.textPrimary,
                  letterSpacing: '0.02em',
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                }}
              >
                {title}
              </h2>
            )}
            {subtitle && (
              <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
                {subtitle}
              </span>
            )}
          </div>

          {headerSlot && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              {headerSlot}
            </div>
          )}

          {showCloseButton && (
            <button
              type="button"
              onClick={onClose}
              aria-label="Close modal"
              style={{
                background: 'transparent',
                border: 'none',
                color: tokens.colors.textMuted,
                cursor: 'pointer',
                padding: '4px',
                borderRadius: '6px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: tokens.transitions.fast,
                flexShrink: 0,
              }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLButtonElement).style.color = tokens.colors.textPrimary;
                (e.currentTarget as HTMLButtonElement).style.background = 'rgba(255, 255, 255, 0.08)';
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLButtonElement).style.color = tokens.colors.textMuted;
                (e.currentTarget as HTMLButtonElement).style.background = 'transparent';
              }}
            >
              <IconX size={18} />
            </button>
          )}
        </div>
      )}

      {/* Main Body Content */}
      <div
        style={{
          flex: 1,
          minHeight: 0,
          overflowY: 'auto',
          scrollbarGutter: 'stable',
          padding: '14px 16px',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <div
          style={{
            maxWidth: maxWidth || 'none',
            width: '100%',
            margin: maxWidth ? '0 auto' : undefined,
            height: '100%',
            minHeight: 0,
            display: 'flex',
            flexDirection: 'column',
            justifyContent: centerContent ? 'center' : 'flex-start',
            gap: '12px',
          }}
        >
          {children}
        </div>
      </div>

      {/* Bottom Sticky Action Footer */}
      {footer && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent:
              footerAlign === 'center'
                ? 'center'
                : footerAlign === 'space-between'
                  ? 'space-between'
                  : 'flex-end',
            gap: '8px',
            padding: '10px 16px',
            background: 'rgba(0, 0, 0, 0.25)',
            borderTop: '1px solid rgba(255, 255, 255, 0.06)',
            flexShrink: 0,
          }}
        >
          {footer}
        </div>
      )}
    </div>
  );
}
