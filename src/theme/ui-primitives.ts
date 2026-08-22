import type { JSX } from 'preact';
import { tokens } from '../design-tokens.ts';

export type Style = JSX.CSSProperties;

export const titleBarHeight = '42px';

export const appShellStyle: Style = {
  display: 'flex',
  flexDirection: 'column',
  width: '100%',
  height: '100%',
  position: 'relative',
  background: 'radial-gradient(ellipse 90% 60% at 50% -10%, rgba(88, 101, 242, 0.18) 0%, rgba(196, 57, 145, 0.12) 40%, transparent 80%), linear-gradient(180deg, #161822 0%, #0f1117 100%)',
  color: tokens.colors.textPrimary,
  border: '1px solid rgba(255, 255, 255, 0.06)',
};

export const titleBarStyle: Style = {
  height: titleBarHeight,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '0 12px',
  background: 'transparent',
  borderBottom: '1px solid rgba(255, 255, 255, 0.05)',
  userSelect: 'none',
  WebkitUserSelect: 'none',
};

export const titleBarTitleStyle: Style = {
  display: 'flex',
  alignItems: 'center',
  fontSize: '14px',
  fontWeight: 700,
  letterSpacing: '0.04em',
  color: tokens.colors.textSecondary,
};

export const titleBarControlsStyle: Style = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  paddingRight: '2px',
};

export const tabNavStyle: Style = {
  display: 'flex',
  padding: '0 8px',
  background: 'transparent',
  borderBottom: 'none',
  alignItems: 'stretch',
};

export const appContentStyle: Style = {
  flex: 1,
  minHeight: 0,
  overflow: 'auto',
};

export const tabPanelStyle: Style = {
  width: '100%',
  minHeight: '100%',
  padding: '12px',
  display: 'flex',
  flexDirection: 'column',
};

export const tabPanelPaddedStyle: Style = {
  width: '100%',
  maxWidth: '900px',
  margin: '0 auto',
  display: 'flex',
  flexDirection: 'column',
  gap: '16px',
};

export const tabPanelContentStyle: Style = {
  width: '100%',
  maxWidth: '900px',
  margin: '0 auto',
  display: 'flex',
  flexDirection: 'column',
};

export const inputBaseStyle: Style = {
  width: '100%',
  background: 'rgba(255, 255, 255, 0.05)',
  color: tokens.colors.textPrimary,
  border: '1px solid rgba(255, 255, 255, 0.1)',
  borderRadius: tokens.radii.input,
  padding: '10px 12px',
  fontSize: tokens.typography.sizeSm,
  outline: 'none',
};

export const selectWrapperStyle: Style = {
  display: 'flex',
  gap: tokens.spacing.sm,
  width: '100%',
  alignItems: 'center',
};

export const helperTextStyle: Style = {
  fontSize: tokens.typography.sizeXs,
  color: '#d9dfe7',
  lineHeight: 1.4,
};

export const toastContainerStyle: Style = {
  position: 'fixed',
  top: '60px',
  left: '50%',
  transform: 'translateX(-50%)',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '8px',
  zIndex: 1000,
  width: 'min(420px, calc(100% - 24px))',
  padding: '0 12px',
  boxSizing: 'border-box',
  pointerEvents: 'none',
};

export const getToastStyle = (type: 'success' | 'error' | 'info' | 'saved'): Style => ({
  width: type === 'saved' ? 'auto' : '100%',
  maxWidth: type === 'saved' ? '220px' : '100%',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  padding: type === 'saved' ? '6px 12px' : '10px 12px',
  borderRadius: '999px',
  border: 'none',
  background: type === 'saved'
    ? '#10b981'
    : type === 'success'
      ? '#10b981'
      : type === 'error'
        ? '#ef4444'
        : '#4cc9f0',
  cursor: type === 'saved' ? 'default' : 'pointer',
  pointerEvents: 'auto',
  boxShadow: type === 'saved' ? '0 3px 10px rgba(0, 0, 0, 0.25)' : '0 4px 12px rgba(0, 0, 0, 0.22)',
});

export const toastDotStyle: Style = {
  width: '8px',
  height: '8px',
  borderRadius: '999px',
  background: tokens.colors.accentPrimary,
  flexShrink: 0,
};

export const toastMessageStyle: Style = {
  fontSize: tokens.typography.sizeSm,
  color: tokens.colors.textPrimary,
};

export const getToastMessageStyle = (type: 'success' | 'error' | 'info' | 'saved'): Style => ({
  fontSize: type === 'saved' ? tokens.typography.sizeXs : tokens.typography.sizeSm,
  color: tokens.colors.textPrimary,
  fontWeight: type === 'saved' ? 700 : 500,
  letterSpacing: type === 'saved' ? '0.01em' : 'normal',
});

export const resizeCornerOverlayStyle: Style = {
  position: 'absolute',
  inset: 0,
  zIndex: 70,
  pointerEvents: 'none',
};

const cornerSize = 40;

function cornerStyle(top: number | string, right: number | string, bottom: number | string, left: number | string): Style {
  return {
    position: 'absolute',
    top,
    right,
    bottom,
    left,
    width: `${cornerSize}px`,
    height: `${cornerSize}px`,
    zIndex: 1,
    userSelect: 'none',
    pointerEvents: 'auto',
    touchAction: 'none',
    background: 'transparent',
  };
}

export const resizeCornerStyles: Record<string, Style> = {
  nw: {
    ...cornerStyle(0, 'auto', 'auto', 0),
    cursor: 'nwse-resize',
    clipPath: 'polygon(0 0, 100% 0, 0 100%)',
  },
  ne: {
    ...cornerStyle(0, 0, 'auto', 'auto'),
    cursor: 'nesw-resize',
    clipPath: 'polygon(0 0, 100% 0, 100% 100%)',
  },
  sw: {
    ...cornerStyle('auto', 'auto', 0, 0),
    cursor: 'nesw-resize',
    clipPath: 'polygon(0 0, 0 100%, 100% 100%)',
  },
  se: {
    ...cornerStyle('auto', 0, 0, 'auto'),
    cursor: 'nwse-resize',
    clipPath: 'polygon(100% 0, 0 100%, 100% 100%)',
  },
};

export const modalTextIntroStyle: Style = {
  ...helperTextStyle,
  marginBottom: '10px',
};

export const modalShortcutPathStyle: Style = {
  fontSize: tokens.typography.sizeSm,
  color: tokens.colors.textPrimary,
  fontWeight: 600,
  marginBottom: '8px',
};

export const modalShortcutNoteStyle: Style = {
  ...helperTextStyle,
};
