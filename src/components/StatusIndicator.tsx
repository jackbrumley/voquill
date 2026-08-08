import { tokens } from '../design-tokens.ts';
import { AudioWave } from './AudioWave.tsx';
import { BouncingDots } from './BouncingDots.tsx';
import { ReadySweep } from './ReadySweep.tsx';

interface StatusIndicatorProps {
  status: string;
  size?: number;
  label?: string;
  subtitle?: string;
}

export function StatusIndicator({ status, size = 40, label, subtitle }: StatusIndicatorProps) {
  const animSize = Math.max(size - 8, 24);

  return (
    <div style={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: label ? '10px' : '0',
      isolation: 'isolate',
      contain: 'paint',
      overflow: 'hidden',
      background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
      border: '1px solid rgba(255, 255, 255, 0.1)',
      borderRadius: '999px',
      padding: label ? `${Math.max(4, Math.round(size * 0.1))}px ${Math.max(10, Math.round(size * 0.25))}px ${Math.max(4, Math.round(size * 0.1))}px ${Math.max(6, Math.round(size * 0.15))}px` : '0',
      minWidth: label ? `${Math.round(size * 2.5)}px` : `${size}px`,
      height: label ? `${size}px` : `${size}px`,
    }}>
      <div style={{
        width: `${animSize}px`,
        height: `${animSize}px`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        flexShrink: 0,
      }}>
        {status === 'Ready' ? (
          <ReadySweep />
        ) : status === 'Recording' ? (
          <AudioWave barWidth={Math.max(2, Math.round(animSize * 0.08))} containerHeight={animSize} gap={Math.max(1, Math.round(animSize * 0.05))} />
        ) : status === 'Transcribing' ? (
          <BouncingDots dotSize={Math.max(4, Math.round(animSize * 0.25))} gap={Math.max(1, Math.round(animSize * 0.05))} jumpHeight={Math.max(4, Math.round(animSize * 0.15))} />
        ) : null}
      </div>
      {label && (
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
            {label}
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
      )}
    </div>
  );
}