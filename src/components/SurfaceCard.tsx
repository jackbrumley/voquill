import { ComponentChildren } from 'preact';
import type { JSX } from 'preact';
import { surfaceCardStyle } from '../theme/component-styles.ts';

interface SurfaceCardProps {
  children: ComponentChildren;
  className?: string;
  style?: JSX.CSSProperties;
}

export const SurfaceCard = ({ children, className = '', style }: SurfaceCardProps) => {
  return <div className={`surface-card ${className}`.trim()} style={{ ...surfaceCardStyle, ...style }}>{children}</div>;
};
