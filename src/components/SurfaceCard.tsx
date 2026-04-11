import { ComponentChildren } from 'preact';

interface SurfaceCardProps {
  children: ComponentChildren;
  className?: string;
}

export const SurfaceCard = ({ children, className = '' }: SurfaceCardProps) => {
  return <div className={`surface-card ${className}`.trim()}>{children}</div>;
};
