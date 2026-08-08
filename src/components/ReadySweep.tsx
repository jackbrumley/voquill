interface ReadySweepProps {
  width?: number | string;
  height?: number | string;
}

export function ReadySweep({ width = '100%', height = '100%' }: ReadySweepProps) {
  return (
    <div style={{
      position: 'relative',
      width,
      height,
      overflow: 'hidden',
    }}>
      <style>{`
        @keyframes voquill-sweep {
          0% { width: 12px; left: 0; }
          25% { width: 100%; left: 0; }
          50% { width: 12px; left: calc(100% - 12px); }
          75% { width: 100%; left: 0; }
          100% { width: 12px; left: 0; }
        }
      `}</style>
      <div style={{
        position: 'absolute',
        top: '50%',
        transform: 'translateY(-50%)',
        width: '12px',
        height: '12px',
        borderRadius: '6px',
        background: 'linear-gradient(90deg, #5ab7d6, rgba(156, 90, 136, 1), #c43991)',
        animation: 'voquill-sweep 3.5s ease-in-out infinite',
      }}></div>
    </div>
  );
}