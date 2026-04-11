
import { ComponentChildren } from 'preact';
import type { JSX } from 'preact';
import { useState } from 'preact/hooks';
import { tokens } from '../design-tokens.ts';

interface CardProps {
  children: ComponentChildren;
  className?: string;
  variant?: 'primary' | 'secondary';
  onClick?: () => void;
  style?: JSX.CSSProperties;
}

export const Card = ({ children, className = '', variant = 'secondary', onClick, style: styleOverride }: CardProps) => {
  const [hovered, setHovered] = useState(false);

  const style = {
    padding: tokens.spacing.lg,
    borderRadius: tokens.radii.panel,
    background: variant === 'primary' ? 'rgba(47, 49, 54, 0.7)' : 'rgba(32, 34, 37, 0.6)',
    backdropFilter: `blur(${tokens.colors.glassBlur})`,
    border: 'none',
    boxShadow: tokens.shadows.md,
    transition: tokens.transitions.normal,
    transform: hovered && onClick ? 'translateY(-2px)' : 'translateY(0)',
    cursor: onClick ? 'pointer' : 'default',
  } as const;

  return (
    <div
      className={className}
      onClick={onClick}
      style={{ ...style, ...styleOverride }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {children}
    </div>
  );
};
