import { ComponentChildren, VNode } from 'preact';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { tokens } from '../design-tokens.ts';

interface ButtonProps {
  children: ComponentChildren;
  onClick?: (e: MouseEvent) => void;
  variant?: 'primary' | 'secondary' | 'configAction' | 'danger' | 'ghost' | 'icon' | 'titlebarIcon' | 'titlebarClose';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  className?: string;
  title?: string;
  type?: 'button' | 'submit' | 'reset';
  style?: Record<string, string | number>;
  logLabel?: string;
  disableClickLog?: boolean;
  pill?: boolean;
  floating?: boolean;
}

function extractText(children: ComponentChildren): string {
  if (typeof children === 'string' || typeof children === 'number') {
    return String(children).trim();
  }

  if (Array.isArray(children)) {
    return children
      .map((child) => extractText(child))
      .filter(Boolean)
      .join(' ')
      .trim();
  }

  if (children && typeof children === 'object') {
    const vnode = children as VNode;
    return extractText(vnode.props?.children);
  }

  return '';
}

export const Button = ({
  children, 
  onClick, 
  variant = 'secondary', 
  size = 'md',
  disabled = false,
  className = '',
  title,
  type = 'button',
  style,
  logLabel,
  disableClickLog = false,
  pill = false,
  floating = false,
}: ButtonProps) => {
  const hovered = useSignal(false);
  const pressed = useSignal(false);

  const variantStyles: Record<string, Record<string, string | number>> = {
    primary: {
      color: tokens.colors.textPrimary,
      background: tokens.colors.success,
      border: 'none',
    },
    secondary: {
      color: tokens.colors.textPrimary,
      background: tokens.colors.accentPrimary,
      border: 'none',
    },
    configAction: {
      color: tokens.colors.textPrimary,
      background: tokens.colors.accentPrimary,
      border: '1px solid rgba(255, 255, 255, 0.08)',
      borderRadius: '999px',
      fontWeight: 600,
      boxShadow: 'none',
    },
    danger: {
      color: tokens.colors.textPrimary,
      background: tokens.colors.error,
      border: 'none',
    },
    ghost: {
      border: '1px solid rgba(255, 255, 255, 0.1)',
      background: 'rgba(255, 255, 255, 0.05)',
      color: tokens.colors.textPrimary,
    },
    icon: {
      border: '1px solid rgba(255, 255, 255, 0.1)',
      background: 'rgba(255, 255, 255, 0.05)',
      color: tokens.colors.textPrimary,
      width: '36px',
      height: '36px',
      padding: 0,
      borderRadius: '999px',
    },
    titlebarIcon: {
      border: '1px solid transparent',
      background: 'rgba(255, 255, 255, 0.1)',
      color: tokens.colors.textPrimary,
      width: '28px',
      height: '28px',
      padding: 0,
      borderRadius: '8px',
      transition: tokens.transitions.fast,
    },
    titlebarClose: {
      border: '1px solid transparent',
      background: 'rgba(239, 68, 68, 0.28)',
      color: tokens.colors.textPrimary,
      width: '28px',
      height: '28px',
      padding: 0,
      borderRadius: '8px',
      transition: tokens.transitions.fast,
    },
  };

  const sizeStyles: Record<string, Record<string, string | number>> = {
    sm: { padding: '5px 12px', fontSize: '11px', gap: '4px' },
    md: { padding: '8px 16px', fontSize: '12.5px', gap: '6px' },
    lg: { padding: '10px 22px', fontSize: '14px', gap: '8px' },
  };

  const hoverStyles: Record<string, Record<string, string | number>> = {
    primary: { background: '#0ea371', transform: 'translateY(-1px)' },
    secondary: { background: tokens.colors.accentHover, transform: 'translateY(-1px)' },
    configAction: { background: tokens.colors.accentHover, filter: 'brightness(1.05)' },
    danger: { background: '#ff5f5f', transform: 'translateY(-1px)' },
    ghost: { background: 'rgba(255, 255, 255, 0.12)', borderColor: 'rgba(255, 255, 255, 0.18)', transform: 'translateY(-1px)' },
    icon: { background: 'rgba(255, 255, 255, 0.12)', borderColor: 'rgba(255, 255, 255, 0.18)', transform: 'translateY(-1px)' },
    titlebarIcon: {
      background: 'rgba(255, 255, 255, 0.3)',
      color: tokens.colors.textPrimary,
    },
    titlebarClose: {
      background: 'rgba(239, 68, 68, 0.58)',
      color: tokens.colors.textPrimary,
    },
  };

  const baseStyle: Record<string, string | number> = {
    background: 'transparent',
    border: '1px solid transparent',
    borderRadius: tokens.radii.button,
    cursor: disabled ? 'not-allowed' : 'pointer',
    fontWeight: 600,
    lineHeight: 1.2,
    whiteSpace: 'nowrap',
    userSelect: 'none',
    WebkitUserSelect: 'none',
    transition: tokens.transitions.fast,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    opacity: disabled ? 0.5 : 1,
    color: disabled ? tokens.colors.textMuted : tokens.colors.textPrimary,
  };

  const resolvedStyle: Record<string, string | number> = {
    ...baseStyle,
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...(hovered.value && !disabled ? hoverStyles[variant] : {}),
    ...(pressed.value && !disabled && !['titlebarIcon', 'titlebarClose'].includes(variant)
      ? { transform: 'translateY(0)', filter: 'brightness(0.9)' }
      : {}),
    ...(pill ? { borderRadius: '999px' } : {}),
    ...(floating
      ? {
          pointerEvents: 'auto',
          padding: '12px 32px',
          borderRadius: '999px',
          backdropFilter: 'blur(12px)',
          WebkitBackdropFilter: 'blur(12px)',
          boxShadow: `${tokens.shadows.lg}, 0 8px 30px rgba(0, 0, 0, 0.5)`,
          border: '1px solid rgba(255, 255, 255, 0.1)',
        }
      : {}),
    ...(floating && hovered.value && !disabled
      ? {
          transform: 'translateY(-4px)',
          boxShadow: `${tokens.shadows.lg}, 0 12px 40px rgba(0, 0, 0, 0.6)`,
          filter: 'brightness(1.1)',
        }
      : {}),
    ...style,
  };

  const handleClick = (e: MouseEvent) => {
    if (!disableClickLog) {
      const label = logLabel || title || extractText(children) || 'Unnamed Button';
      invoke('log_ui_event', { message: `[Button clicked] ${label}` }).catch(() => {});
    }
    onClick?.(e);
  };

  return (
    <button
      type={type}
      className={className}
      onClick={handleClick}
      disabled={disabled}
      title={title}
      style={resolvedStyle}
      onMouseEnter={() => { hovered.value = true; }}
      onMouseLeave={() => {
        hovered.value = false;
        pressed.value = false;
      }}
      onMouseDown={() => { pressed.value = true; }}
      onMouseUp={() => { pressed.value = false; }}
    >
      {children}
    </button>
  );
};
