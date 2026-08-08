import { useSignal } from '@preact/signals';

const wrapperStyle: Record<string, string | number> = {
  background: '#1f2125',
  borderRadius: '12px',
  padding: '12px',
  border: '1px solid rgba(255,255,255,0.07)',
  cursor: 'pointer',
  transition: 'border-color 0.2s, background 0.2s',
};

interface CardProps {
  children: preact.ComponentChildren;
  style?: Record<string, string | number | undefined>;
  className?: string;
}

export function Card({ children, style, className }: CardProps) {
  const hovered = useSignal(false);

  return (
    <div
      className={className}
      style={{
        ...wrapperStyle,
        ...style,
        borderColor: hovered.value ? 'rgba(255,255,255,0.15)' : (style?.borderColor || 'rgba(255,255,255,0.07)'),
        background: hovered.value ? 'rgba(255,255,255,0.03)' : (style?.background || '#1f2125'),
      }}
      onMouseEnter={() => { hovered.value = true; }}
      onMouseLeave={() => { hovered.value = false; }}
    >
      {children}
    </div>
  );
}