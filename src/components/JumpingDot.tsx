interface JumpingDotProps {
  dotSize?: number;
  jumpHeight?: number;
}

export function JumpingDot({ dotSize = 36, jumpHeight = 52 }: JumpingDotProps) {
  return (
    <div style={{
      width: '100%',
      height: '100%',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      '--jump-height': `-${jumpHeight}px`,
    } as Record<string, string | number>}>
      <style>{`
        @keyframes voquill-ready-dot-jump {
          0%, 40%, 100% { transform: translateY(0) scale(1, 1); }
          43% { transform: translateY(0) scale(1.25, 0.75); }
          53% { transform: translateY(var(--jump-height)) scale(0.9, 1.1); }
          63% { transform: translateY(0) scale(1.2, 0.8); }
          68% { transform: translateY(calc(var(--jump-height) / 4)) scale(0.95, 1.05); }
          73% { transform: translateY(0) scale(1.05, 0.95); }
        }
      `}</style>
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        transform: 'translateY(calc(var(--jump-height) / -2))',
      }}>
        <span style={{
          width: `${dotSize}px`,
          height: `${dotSize}px`,
          borderRadius: '50%',
          background: 'linear-gradient(135deg, #5ab7d6 15%, rgba(123, 120, 163, 1) 34%, rgba(156, 90, 136, 1) 54%, #c43991 85%)',
          boxShadow: '0 4px 10px rgba(0, 0, 0, 0.3), 0 0 12px rgba(196, 57, 145, 0.35)',
          animation: 'voquill-ready-dot-jump 3s infinite ease-in-out',
        }}></span>
      </div>
    </div>
  );
}
