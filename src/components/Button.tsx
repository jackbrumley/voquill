
import { ComponentChildren, VNode } from 'preact';
import { invoke } from '@tauri-apps/api/core';

interface ButtonProps {
  children: ComponentChildren;
  onClick?: (e: MouseEvent) => void;
  variant?: 'primary' | 'secondary' | 'configAction' | 'danger' | 'ghost' | 'icon';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  className?: string;
  title?: string;
  type?: 'button' | 'submit' | 'reset';
  style?: any;
  logLabel?: string;
  disableClickLog?: boolean;
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
}: ButtonProps) => {
  const classes = [
    'btn',
    `btn-${variant}`,
    `btn-${size}`,
    className
  ].join(' ');

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
      className={classes} 
      onClick={handleClick}
      disabled={disabled}
      title={title}
      style={style}
    >
      {children}
    </button>
  );
};
