import { tokens } from '../design-tokens.ts';

interface RatingDotsProps {
  label: string;
  /** 1 to `max`. */
  value: number;
  max?: number;
}

/// A label with filled and empty dots, e.g. "Speed ●●●○".
export function RatingDots({ label, value, max = 4 }: RatingDotsProps) {
  return (
    <span
      title={`${label}: ${value} of ${max}`}
      style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}
    >
      {label}
      <span aria-hidden="true" style={{ letterSpacing: '1px' }}>
        {Array.from({ length: max }, (_, index) => (
          <span key={index} style={{ color: index < value ? '#818cf8' : 'rgba(255, 255, 255, 0.2)' }}>●</span>
        ))}
      </span>
    </span>
  );
}
