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
    borderRadius: '14px',
    background: variant === 'primary' ? 'rgba(36, 40, 52, 0.75)' : 'rgba(26, 29, 38, 0.65)',
    backdropFilter: `blur(${tokens.colors.glassBlur})`,
    border: '1px solid rgba(255, 255, 255, 0.08)',
    boxShadow: '0 6px 24px rgba(0, 0, 0, 0.35), 0 2px 6px rgba(0, 0, 0, 0.25)',
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