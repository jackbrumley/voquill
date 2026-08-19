import { tokens } from '../design-tokens.ts';
import { statusLabel } from '../status.ts';
import type { DictationStatus } from '../types.ts';
import { StatusAnimation } from './StatusAnimation.tsx';

interface StatusIndicatorProps {
  status: DictationStatus;
  size?: number;
  label?: string;
  subtitle?: string;
  fixedWidth?: number;
}

export function StatusIndicator({ status, size = 40, label, subtitle, fixedWidth }: StatusIndicatorProps) {
  const animSize = Math.max(size - 8, 24);
  const resolvedLabel = label ?? statusLabel(status);

  return (
    <div style={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: '10px',
      isolation: 'isolate',
      contain: 'paint',
      overflow: 'hidden',
      background: `radial-gradient(circle 38px at ${Math.max(16, Math.round(size * 0.5))}px 50%, rgba(88, 101, 242, 0.3) 0%, rgba(196, 57, 145, 0.15) 50%, transparent 80%), #151822`,
      border: '1px solid rgba(255, 255, 255, 0.08)',
      boxShadow: '0 6px 20px rgba(0, 0, 0, 0.5), 0 2px 6px rgba(0, 0, 0, 0.3)',
      borderRadius: '999px',
      padding: `${Math.max(4, Math.round(size * 0.1))}px ${Math.max(10, Math.round(size * 0.25))}px ${Math.max(4, Math.round(size * 0.1))}px ${Math.max(6, Math.round(size * 0.15))}px`,
      minWidth: fixedWidth ? `${fixedWidth}px` : `${Math.round(size * 2.5)}px`,
      width: fixedWidth ? `${fixedWidth}px` : undefined,
      height: `${size}px`,
    }}>
      <StatusAnimation status={status} size={animSize} />
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        flex: 1,
        minWidth: 0,
      }}>
        <span style={{
          color: '#fff',
          fontFamily: tokens.typography.fontMain,
          fontSize: `${Math.max(12, Math.round(size * 0.38))}px`,
          fontWeight: 500,
          lineHeight: 1.2,
          whiteSpace: 'nowrap',
          textShadow: 'none',
        }}>
          {resolvedLabel}
        </span>
        {subtitle && (
          <span style={{
            fontSize: `${Math.max(9, Math.round(size * 0.22))}px`,
            opacity: 0.75,
            color: '#fff',
            whiteSpace: 'nowrap',
          }}>
            {subtitle}
          </span>
        )}
      </div>
    </div>
  );
}
