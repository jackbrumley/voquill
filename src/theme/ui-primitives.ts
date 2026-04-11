import type { JSX } from 'preact';
import { tokens } from '../design-tokens.ts';

export type Style = JSX.CSSProperties;

export const appShellStyle: Style = {
  display: 'flex',
  flexDirection: 'column',
  width: '100%',
  height: '100%',
  position: 'relative',
  background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
  color: tokens.colors.textPrimary,
};

export const titleBarStyle: Style = {
  height: '42px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '0 12px',
  background: tokens.colors.bgTertiary,
  backdropFilter: 'blur(10px)',
  borderBottom: '1px solid rgba(255, 255, 255, 0.06)',
  userSelect: 'none',
};

export const titleBarTitleStyle: Style = {
  fontSize: '13px',
  fontWeight: 700,
  letterSpacing: '0.04em',
  color: tokens.colors.textSecondary,
};

export const titleBarControlsStyle: Style = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
};

export const tabNavStyle: Style = {
  display: 'flex',
  gap: '4px',
  padding: '8px 8px 0 8px',
  background: 'rgba(47, 49, 54, 0.8)',
  backdropFilter: 'blur(10px)',
  WebkitBackdropFilter: 'blur(10px)',
  border: 'none',
  alignItems: 'stretch',
};

export const tabContentStyle: Style = {
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

export const selectBaseStyle: Style = {
  ...inputBaseStyle,
  appearance: 'none',
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
  right: '14px',
  bottom: '14px',
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  zIndex: 1000,
};

export const getToastStyle = (type: 'success' | 'error' | 'info'): Style => ({
  minWidth: '240px',
  maxWidth: '420px',
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '10px 12px',
  borderRadius: '10px',
  border: '1px solid rgba(255, 255, 255, 0.1)',
  backdropFilter: 'blur(12px)',
  WebkitBackdropFilter: 'blur(12px)',
  background: type === 'success'
    ? 'rgba(16, 185, 129, 0.14)'
    : type === 'error'
      ? 'rgba(239, 68, 68, 0.14)'
      : 'rgba(47, 49, 54, 0.9)',
  cursor: 'pointer',
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
