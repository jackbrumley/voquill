
import { ComponentChildren } from 'preact';

interface ButtonProps {
  children: ComponentChildren;
  onClick?: (e: MouseEvent) => void;
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'icon';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  className?: string;
  title?: string;
  type?: 'button' | 'submit' | 'reset';
  style?: any;
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
  style
}: ButtonProps) => {
  const classes = [
    'btn',
    `btn-${variant}`,
    `btn-${size}`,
    className
  ].join(' ');

  return (
    <button 
      type={type}
      className={classes} 
      onClick={onClick} 
      disabled={disabled}
      title={title}
      style={style}
    >
      {children}
    </button>
  );
};
