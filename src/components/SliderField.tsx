import type { JSX } from 'preact';
import { tokens } from '../design-tokens.ts';

export interface SliderFieldProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  ariaLabel?: string;
  id?: string;
  formatEndLabel?: (value: number) => string;
  hideEndLabels?: boolean;
  style?: JSX.CSSProperties;
}

export function SliderField({
  value,
  min = 0,
  max = 1,
  step = 0.01,
  disabled = false,
  onChange,
  ariaLabel,
  id,
  formatEndLabel,
  hideEndLabels = false,
  style,
}: SliderFieldProps) {
  const rangeSpan = max - min;
  const ratio = rangeSpan > 0 ? Math.min(Math.max((value - min) / rangeSpan, 0), 1) : 0;
  const percent = ratio * 100;
  // Thumb width is 16px. Offset correction centers the color stop under the grab handle.
  const offsetPx = (0.5 - ratio) * 16;
  const fillStop = `calc(${percent}% + ${offsetPx.toFixed(2)}px)`;

  const handleInput = (event: Event) => {
    if (disabled) return;
    const target = event.currentTarget as HTMLInputElement;
    const parsed = parseFloat(target.value);
    if (!Number.isNaN(parsed)) {
      onChange(parsed);
    }
  };

  const formatDefaultLabel = (val: number): string => {
    if (min >= 0 && max <= 1 && step < 0.01) {
      return `${(val * 100).toFixed(1)}%`;
    }
    if (min >= 0 && max <= 1) {
      return `${Math.round(val * 100)}%`;
    }
    return `${val}`;
  };

  const minLabel = formatEndLabel ? formatEndLabel(min) : formatDefaultLabel(min);
  const maxLabel = formatEndLabel ? formatEndLabel(max) : formatDefaultLabel(max);

  const inputStyle: Record<string, string | number> = {
    '--slider-fill': fillStop,
    ...(style as Record<string, string | number> | undefined),
  };

  return (
    <div style={{ width: '100%', display: 'flex', flexDirection: 'column' }}>
      <input
        type="range"
        id={id}
        className="voquill-slider"
        value={value}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        onInput={handleInput}
        aria-label={ariaLabel}
        style={inputStyle}
      />
      {!hideEndLabels && (
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            fontSize: tokens.typography.sizeXs,
            color: tokens.colors.textMuted,
            marginTop: '-2px',
            userSelect: 'none',
          }}
        >
          <span>{minLabel}</span>
          <span>{maxLabel}</span>
        </div>
      )}
    </div>
  );
}
