import { ComponentChildren } from 'preact';
import type { JSX } from 'preact';
import { useSignal } from '@preact/signals';
import { tokens } from '../design-tokens.ts';

interface CardProps {
  children: ComponentChildren;
  className?: string;
  variant?: 'primary' | 'secondary';
  onClick?: () => void;
  style?: JSX.CSSProperties;
}

export const Card = ({ children, className = '', variant = 'secondary', onClick, style: styleOverride }: CardProps) => {
  const hovered = useSignal(false);

  const style = {
    padding: tokens.spacing.lg,
    borderRadius: tokens.radii.panel,
    background: variant === 'primary' ? 'rgba(47, 49, 54, 0.7)' : 'rgba(32, 34, 37, 0.6)',
    backdropFilter: `blur(${tokens.colors.glassBlur})`,
    border: 'none',
    boxShadow: tokens.shadows.md,
    transition: tokens.transitions.normal,
    transform: hovered.value && onClick ? 'translateY(-2px)' : 'translateY(0)',
    cursor: onClick ? 'pointer' : 'default',
  } as const;

  return (
    <div
      className={className}
      style={{ ...style, ...styleOverride }}
      onClick={onClick}
      onMouseEnter={() => { hovered.value = true; }}
      onMouseLeave={() => { hovered.value = false; }}
    >
      {children}
    </div>
  );
};