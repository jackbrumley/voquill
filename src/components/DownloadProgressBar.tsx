import type { JSX } from 'preact';
import type { DownloadPhase } from '../types.ts';
import { tokens } from '../design-tokens.ts';

export interface DownloadProgressBarProps {
  isDownloading: boolean;
  progress: number;
  phase?: DownloadPhase | string;
  itemLabel?: string;
  compact?: boolean;
  style?: JSX.CSSProperties;
}

export function DownloadProgressBar({
  isDownloading,
  progress,
  phase = 'downloading',
  itemLabel = 'model',
  compact = false,
  style,
}: DownloadProgressBarProps) {
  if (!isDownloading) {
    return null;
  }

  const clampedProgress = Math.max(0, Math.min(100, progress));
  const isExtracting = phase === 'extracting';

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: compact ? '3px' : '5px',
        width: '100%',
        ...style,
      }}
    >
      <div
        style={{
          width: '100%',
          height: compact ? '3px' : '4px',
          background: tokens.colors.bgTertiary,
          borderRadius: tokens.radii.button,
          overflow: 'hidden',
          border: '1px solid rgba(255, 255, 255, 0.04)',
        }}
      >
        <div
          style={{
            width: isExtracting ? '100%' : `${clampedProgress}%`,
            height: '100%',
            background: isExtracting
              ? 'linear-gradient(90deg, #10b981 0%, #059669 50%, #10b981 100%)'
              : tokens.colors.success,
            borderRadius: tokens.radii.button,
            transition: 'width 200ms ease-out',
            animation: isExtracting ? 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite' : undefined,
          }}
        />
      </div>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          fontSize: compact ? '10px' : '11px',
          color: tokens.colors.textSecondary,
        }}
      >
        <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          {isExtracting
            ? `Extracting ${itemLabel}... this can take a moment`
            : `Downloading ${itemLabel}...`}
        </span>
        <span
          style={{
            fontWeight: 600,
            color: tokens.colors.textPrimary,
            fontFamily: tokens.typography.fontMono,
          }}
        >
          {isExtracting ? '100%' : `${Math.round(clampedProgress)}%`}
        </span>
      </div>
    </div>
  );
}
