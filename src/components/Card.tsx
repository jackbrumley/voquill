
import { ComponentChildren } from 'preact';

interface CardProps {
  children: ComponentChildren;
  className?: string;
  variant?: 'primary' | 'secondary';
  onClick?: () => void;
}

export const Card = ({ children, className = '', variant = 'secondary', onClick }: CardProps) => {
  const baseClass = variant === 'primary' ? 'card-primary' : 'card-secondary';
  return (
    <div 
      className={`card ${baseClass} ${className} ${onClick ? 'clickable' : ''}`} 
      onClick={onClick}
    >
      {children}
    </div>
  );
};
