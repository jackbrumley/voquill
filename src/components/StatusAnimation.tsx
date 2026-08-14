import type { DictationStatus } from '../types.ts';
import { AudioWave } from './AudioWave.tsx';
import { BouncingDots } from './BouncingDots.tsx';
import { ReadySweep } from './ReadySweep.tsx';

interface StatusAnimationProps {
  status: DictationStatus;
  size: number;
}

export function StatusAnimation({ status, size }: StatusAnimationProps) {
  return (
    <div style={{
      width: `${size}px`,
      height: `${size}px`,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      flexShrink: 0,
    }}>
      {status === 'Ready' ? (
        <ReadySweep />
      ) : status === 'Recording' ? (
        <AudioWave barWidth={Math.max(2, Math.round(size * 0.08))} containerHeight={size} gap={Math.max(1, Math.round(size * 0.05))} />
      ) : status === 'Transcribing' || status === 'Processing' || status === 'Typing' ? (
        <BouncingDots dotSize={Math.max(4, Math.round(size * 0.25))} gap={Math.max(1, Math.round(size * 0.05))} jumpHeight={Math.max(4, Math.round(size * 0.15))} />
      ) : null}
    </div>
  );
}
