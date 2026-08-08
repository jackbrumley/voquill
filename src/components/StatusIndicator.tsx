import { AudioWave } from './AudioWave.tsx';
import { BouncingDots } from './BouncingDots.tsx';
import { ReadySweep } from './ReadySweep.tsx';

interface StatusIndicatorProps {
  status: string;
  size?: number;
}

export function StatusIndicator({ status, size = 40 }: StatusIndicatorProps) {
  return (
    <div style={{
      width: `${size}px`,
      height: `${size}px`,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}>
      {status === 'Ready' ? (
        <ReadySweep />
      ) : status === 'Recording' ? (
        <AudioWave barWidth={3} containerHeight={size} gap={2} />
      ) : status === 'Transcribing' ? (
        <BouncingDots dotSize={Math.round(size * 0.3)} gap={2} jumpHeight={Math.round(size * 0.2)} />
      ) : null}
    </div>
  );
}