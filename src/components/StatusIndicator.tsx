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
      background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
      border: '1px solid rgba(255, 255, 255, 0.1)',
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
