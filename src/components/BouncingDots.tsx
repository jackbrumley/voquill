interface BouncingDotsProps {
  dotSize?: number;
  gap?: number;
  height?: number | string;
}

export function BouncingDots({ dotSize = 10, gap = 4, height = '100%' }: BouncingDotsProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: `${gap}px`,
      height,
    }}>
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