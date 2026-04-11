
import { ComponentChildren, VNode } from 'preact';
import { useState } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import { tokens } from '../design-tokens.ts';

interface ButtonProps {
  children: ComponentChildren;
  onClick?: (e: MouseEvent) => void;
  variant?: 'primary' | 'secondary' | 'configAction' | 'danger' | 'ghost' | 'icon';
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
    const vnode = children as VNode<any>;
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
  const [hovered, setHovered] = useState(false);
  const [pressed, setPressed] = useState(false);

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
      borderRadius: '40px',
      padding: '10px 24px',
      fontWeight: 700,
      boxShadow: 'none',
    },
    danger: {
      color: tokens.colors.textPrimary,
      background: tokens.colors.error,
      border: 'none',
    },
    ghost: {
      border: '1px solid transparent',
      background: 'rgba(255, 255, 255, 0.08)',
      color: '#d9dfe7',
    },
    icon: {
      border: '1px solid transparent',
      background: 'rgba(255, 255, 255, 0.08)',
      color: tokens.colors.textPrimary,
      width: '38px',
      height: '38px',
      padding: tokens.spacing.sm,
      borderRadius: '999px',
    },
  };

  const sizeStyles: Record<string, Record<string, string | number>> = {
    sm: { padding: `6px ${tokens.spacing.sm}`, fontSize: tokens.typography.sizeXs },
    md: { padding: `10px ${tokens.spacing.md}`, fontSize: tokens.typography.sizeSm },
    lg: { padding: `14px ${tokens.spacing.lg}`, fontSize: tokens.typography.sizeMd },
  };

  const hoverStyles: Record<string, Record<string, string | number>> = {
    primary: { background: '#0ea371', transform: 'translateY(-2px)' },
    secondary: { background: tokens.colors.accentHover, transform: 'translateY(-2px)' },
    configAction: { background: tokens.colors.accentHover, filter: 'brightness(1.04)' },
    danger: { background: '#ff5f5f', transform: 'translateY(-2px)' },
    ghost: { background: 'rgba(255, 255, 255, 0.14)', transform: 'translateY(-1px)' },
    icon: { background: 'rgba(255, 255, 255, 0.14)', transform: 'translateY(-1px)' },
  };

  const baseStyle: Record<string, string | number> = {
    background: 'transparent',
    border: '2px solid transparent',
    borderRadius: tokens.radii.button,
    cursor: disabled ? 'not-allowed' : 'pointer',
    fontWeight: 600,
    transition: tokens.transitions.normal,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: tokens.spacing.sm,
    opacity: disabled ? 0.5 : 1,
    color: disabled ? tokens.colors.textMuted : tokens.colors.textPrimary,
  };

  const resolvedStyle: Record<string, string | number> = {
    ...baseStyle,
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...(hovered && !disabled ? hoverStyles[variant] : {}),
    ...(pressed && !disabled ? { transform: 'translateY(0)', filter: 'brightness(0.9)' } : {}),
    ...(pill ? { borderRadius: '40px' } : {}),
    ...(floating
      ? {
          pointerEvents: 'auto',
          padding: '12px 32px',
          borderRadius: '40px',
          backdropFilter: 'blur(12px)',
          WebkitBackdropFilter: 'blur(12px)',
          boxShadow: `${tokens.shadows.lg}, 0 8px 30px rgba(0, 0, 0, 0.5)`,
          border: '1px solid rgba(255, 255, 255, 0.1)',
        }
      : {}),
    ...(floating && hovered && !disabled
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
      invoke('log_ui_event', { message: `🖱️ Button clicked: ${label}` }).catch(() => {});
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
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => {
        setHovered(false);
        setPressed(false);
      }}
      onMouseDown={() => setPressed(true)}
      onMouseUp={() => setPressed(false)}
    >
      {children}
    </button>
  );
};
