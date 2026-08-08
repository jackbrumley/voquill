interface BouncingDotsProps {
  dotSize?: number;
  gap?: number;
  height?: number | string;
  jumpHeight?: number;
}

export function BouncingDots({ dotSize = 12, gap = 4, height = '100%', jumpHeight = 14 }: BouncingDotsProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: `${gap}px`,
      height,
      '--jump': `${jumpHeight}px`,
    } as Record<string, string | number | undefined>}>
      <style>{`
        @keyframes voquill-bounce-dot {
          0% { transform: translateY(calc(-1 * var(--jump, 14px))); }
          50% { transform: translateY(var(--jump, 14px)); }
          100% { transform: translateY(calc(-1 * var(--jump, 14px))); }
        }
      `}</style>
      <div style={{
        width: `${dotSize}px`,
        height: `${dotSize}px`,
        borderRadius: '50%',
        background: '#5ab7d6',
        animation: 'voquill-bounce-dot 1.6s ease-in-out infinite',
        animationDelay: '0s',
      }}></div>
      <div style={{
        width: `${dotSize}px`,
        height: `${dotSize}px`,
        borderRadius: '50%',
        background: 'rgba(156, 90, 136, 1)',
        animation: 'voquill-bounce-dot 1.6s ease-in-out infinite',
        animationDelay: '0.2s',
      }}></div>
      <div style={{
        width: `${dotSize}px`,
        height: `${dotSize}px`,
        borderRadius: '50%',
        background: '#c43991',
        animation: 'voquill-bounce-dot 1.6s ease-in-out infinite',
        animationDelay: '0.4s',
      }}></div>
    </div>
  );
}