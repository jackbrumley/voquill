import { useSignal } from '@preact/signals';
import { tokens } from '../design-tokens.ts';

const baseStyle: Record<string, string | number> = {
  background: tokens.colors.bgTertiary,
  border: `1px solid rgba(255, 255, 255, 0.1)`,
  borderRadius: tokens.radii.input,
  color: tokens.colors.textPrimary,
  fontSize: tokens.typography.sizeSm,
  padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`,
  cursor: 'pointer',
  transition: 'border-color 0.15s, background 0.15s',
  outline: 'none',
  width: '100%',
  boxSizing: 'border-box' as const,
};

const hoverStyle = {
  ...baseStyle,
  borderColor: 'rgba(255,255,255,0.2)',
  background: 'rgba(255,255,255,0.06)',
};

interface ButtonProps {
  children: preact.ComponentChildren;
  onClick?: (e: MouseEvent) => void;
  style?: Record<string, string | number>;
  disabled?: boolean;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'icon' | 'titlebarIcon' | 'titlebarClose' | 'configAction';
  pill?: boolean;
  floating?: boolean;
  size?: string;
  title?: string;
}

export function Button({ children, onClick, style, disabled, variant = 'primary', pill, floating, size, title }: ButtonProps) {
  const hovered = useSignal(false);
  const pressed = useSignal(false);

  const base: Record<string, string | number> = {
    cursor: disabled ? 'not-allowed' : 'pointer',
    opacity: disabled ? 0.5 : 1,
    transition: 'all 0.15s',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '6px',
    outline: 'none',
  };

  if (variant === 'icon') {
    Object.assign(base, {
      background: hovered.value ? 'rgba(255,255,255,0.08)' : 'transparent',
      border: 'none',
      borderRadius: '8px',
      padding: '6px',
      color: '#fff',
      width: size || '28px',
      height: size || '28px',
    });
  } else if (variant === 'titlebarIcon') {
    Object.assign(base, {
      background: hovered.value ? 'rgba(255,255,255,0.1)' : 'transparent',
      border: 'none',
      borderRadius: 0,
      padding: '0 14px',
      color: '#fff',
      height: '100%',
      minWidth: '46px',
    });
  } else if (variant === 'titlebarClose') {
    Object.assign(base, {
      background: hovered.value ? '#e81123' : 'transparent',
      border: 'none',
      borderRadius: 0,
      padding: '0 14px',
      color: '#fff',
      height: '100%',
      minWidth: '46px',
    });
  } else {
    const isColored = variant === 'primary' || variant === 'secondary' || variant === 'danger';
    Object.assign(base, hovered.value ? hoverStyle : baseStyle, {
      fontWeight: variant === 'ghost' ? 400 : 600,
      background: variant === 'danger' ? '#e74c3c' : variant === 'primary' ? tokens.colors.accentPrimary : isColored ? tokens.colors.accentPrimary : hovered.value ? hoverStyle.background : baseStyle.background,
      border: isColored ? 'none' : baseStyle.border,
      borderRadius: pill ? '999px' : tokens.radii.input,
      padding: floating ? '16px 32px' : `${tokens.spacing.xs} ${tokens.spacing.sm}`,
      boxShadow: pressed.value && !disabled ? 'inset 0 2px 4px rgba(0,0,0,0.3)' : 'none',
    });
  }

  return (
    <button
      style={{ ...base, ...style } as Record<string, string | number>}
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
      title={title}
      onMouseEnter={() => { hovered.value = true; }}
      onMouseLeave={() => { hovered.value = false; pressed.value = false; }}
      onMouseDown={() => { pressed.value = true; }}
      onMouseUp={() => { pressed.value = false; }}
    >
      {children}
    </button>
  );
}