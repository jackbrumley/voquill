
const bars = [
  { h: 287.9, delay: '0s', color: '#5ab7d6' },
  { h: 680.1, delay: '0.1s', color: 'rgba(90, 183, 214, 1)' },
  { h: 493.6, delay: '0.2s', color: 'rgba(123, 120, 163, 1)' },
  { h: 894.2, delay: '0.3s', color: 'rgba(156, 90, 136, 1)' },
  { h: 625.7, delay: '0.4s', color: 'rgba(196, 57, 145, 1)' },
  { h: 292.8, delay: '0.5s', color: 'rgba(196, 57, 145, 1)' },
  { h: 176.1, delay: '0.6s', color: '#c43991' },
];

interface AudioWaveProps {
  barWidth?: number;
  containerHeight?: number;
  gap?: number;
}

export function AudioWave({ barWidth = 8, containerHeight = 120, gap = 4 }: AudioWaveProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: `${containerHeight}px`,
      padding: '0',
      gap: `${gap}px`,
    }}>
      {bars.map((bar, i) => {
        const pct = (bar.h / 894.2) * 100;
        return (
          <div
            key={i}
            style={{
              width: `${barWidth}px`,
              height: `${pct}%`,
              background: bar.color,
              transformOrigin: 'center center',
              animation: `voquill-wave-bar 1.6s ease-in-out infinite`,
              animationDelay: bar.delay,
            }}
          ></div>
        );
      })}
    </div>
  );
}