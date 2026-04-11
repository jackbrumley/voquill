import type { JSX } from 'preact';
import { tokens } from '../design-tokens.ts';

type Style = JSX.CSSProperties;

export const surfaceCardStyle: Style = {
  background: tokens.colors.glassBg,
  backdropFilter: `blur(${tokens.colors.glassBlur})`,
  WebkitBackdropFilter: `blur(${tokens.colors.glassBlur})`,
  border: '1px solid rgba(255, 255, 255, 0.1)',
  boxShadow: tokens.shadows.md,
};

export const settingRowBaseStyle: Style = {
  marginBottom: tokens.spacing.md,
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  alignItems: 'flex-start',
  border: '1px solid rgba(255, 255, 255, 0.08)',
  borderRadius: '10px',
  background: 'rgba(255, 255, 255, 0.02)',
  padding: '12px 14px',
  transition: 'border-color 0.2s ease, background 0.2s ease',
};

export const getSettingRowStyle = ({
  ready,
}: {
  ready: boolean;
}): Style => {
  if (ready) {
    return {
      ...settingRowBaseStyle,
      background: 'rgba(16, 185, 129, 0.05)',
      borderColor: 'rgba(16, 185, 129, 0.2)',
    };
  }

  return settingRowBaseStyle;
};

export const settingRowHeaderStyle: Style = {
  width: '100%',
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'flex-start',
  gap: tokens.spacing.sm,
};

export const settingRowStatusStyle: Style = {
  marginLeft: 'auto',
  flexShrink: 0,
};

export const settingRowLabelStyle: Style = {
  fontWeight: 600,
  color: tokens.colors.textPrimary,
  fontSize: tokens.typography.sizeSm,
  width: '100%',
  display: 'block',
  textAlign: 'left',
};

export const settingRowDescriptionStyle: Style = {
  fontSize: tokens.typography.sizeXs,
  color: '#d9dfe7',
  margin: `0 0 ${tokens.spacing.sm} 0`,
  lineHeight: 1.4,
  textAlign: 'left',
};

export const settingRowContentStyle: Style = {
  display: 'flex',
  flexDirection: 'column',
  gap: tokens.spacing.xs,
  width: '100%',
  alignItems: 'center',
};
