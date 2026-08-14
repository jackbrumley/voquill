import { tokens } from '../design-tokens.ts';
import { statusLabel } from '../status.ts';
import type { DictationStatus } from '../types.ts';
import { StatusAnimation } from './StatusAnimation.tsx';

interface StatusReadoutProps {
  status: DictationStatus;
  width?: number;
  height?: number;
}

export function StatusReadout({ status, width = 180, height = 56 }: StatusReadoutProps) {
  const isError = status === 'Error';

  return (
    <div style={{
      width: `${width}px`,
      height: `${height}px`,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: '6px',
      flexShrink: 0,
    }}>
      <StatusAnimation status={status} size={32} />
      <span style={{
        color: isError ? tokens.colors.error : tokens.colors.textSecondary,
        fontFamily: tokens.typography.fontMain,
        fontSize: tokens.typography.sizeSm,
        fontWeight: 500,
        lineHeight: 1.2,
        whiteSpace: 'nowrap',
      }}>
        {statusLabel(status)}
      </span>
    </div>
  );
}
