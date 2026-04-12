import type { JSX } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import { tokens } from '../design-tokens.ts';
import { inputBaseStyle } from '../theme/ui-primitives.ts';

interface SliderFieldProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  ariaLabel?: string;
  style?: JSX.CSSProperties;
}

export function SliderField({
  value,
  min,
  max,
  step,
  onChange,
  ariaLabel,
  style,
}: SliderFieldProps) {
  const minPercent = Math.round(min * 100);
  const maxPercent = Math.round(max * 100);
  const currentPercent = Math.round(value * 100);
  const [draftPercent, setDraftPercent] = useState(String(currentPercent));

  useEffect(() => {
    setDraftPercent(String(currentPercent));
  }, [currentPercent]);

  const clampPercent = (nextPercent: number) => Math.min(maxPercent, Math.max(minPercent, nextPercent));

  const commitPercentValue = (rawValue: string) => {
    if (rawValue.trim() === '') {
      setDraftPercent(String(currentPercent));
      return;
    }

    const parsedPercent = Number(rawValue);
    if (Number.isNaN(parsedPercent)) {
      setDraftPercent(String(currentPercent));
      return;
    }

    const clampedPercent = clampPercent(Math.round(parsedPercent));
    setDraftPercent(String(clampedPercent));
    onChange(clampedPercent / 100);
  };

  return (
    <>
      <style>{`
        .voquill-slider-field {
          -webkit-appearance: none;
          appearance: none;
          width: 100%;
          height: 6px;
          border-radius: 3px;
          background: ${tokens.colors.bgTertiary};
          outline: none;
        }

        .voquill-slider-field::-webkit-slider-runnable-track {
          height: 6px;
          border-radius: 3px;
          background: ${tokens.colors.bgTertiary};
        }

        .voquill-slider-field::-webkit-slider-thumb {
          -webkit-appearance: none;
          appearance: none;
          width: 14px;
          height: 14px;
          border-radius: 999px;
          background: ${tokens.colors.accentPrimary};
          border: none;
          box-shadow: none;
          margin-top: -4px;
          cursor: pointer;
        }

        .voquill-slider-field::-moz-range-track {
          height: 6px;
          border-radius: 3px;
          background: ${tokens.colors.bgTertiary};
        }

        .voquill-slider-field::-moz-range-thumb {
          width: 14px;
          height: 14px;
          border-radius: 999px;
          background: ${tokens.colors.accentPrimary};
          border: none;
          box-shadow: none;
          cursor: pointer;
        }

        .voquill-slider-percent {
          -moz-appearance: textfield;
          appearance: textfield;
        }

        .voquill-slider-percent::-webkit-outer-spin-button,
        .voquill-slider-percent::-webkit-inner-spin-button {
          -webkit-appearance: none;
          margin: 0;
        }
      `}</style>
      <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.sm, width: '100%', ...style }}>
        <input
          className="voquill-slider-field"
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onInput={(event: Event) => {
            const target = event.target as HTMLInputElement;
            const nextValue = parseFloat(target.value);
            onChange(nextValue);
            setDraftPercent(String(Math.round(nextValue * 100)));
          }}
          onChange={(event: Event) => {
            const target = event.target as HTMLInputElement;
            const nextValue = parseFloat(target.value);
            onChange(nextValue);
            setDraftPercent(String(Math.round(nextValue * 100)));
          }}
          aria-label={ariaLabel}
          style={{ flex: 1, minWidth: 0 }}
        />
        <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.xs, flexShrink: 0 }}>
          <input
            className="voquill-slider-percent"
            type="number"
            min={minPercent}
            max={maxPercent}
            step={1}
            value={draftPercent}
            onInput={(event: Event) => {
              setDraftPercent((event.target as HTMLInputElement).value);
            }}
            onBlur={(event: Event) => {
              commitPercentValue((event.target as HTMLInputElement).value);
            }}
            onKeyDown={(event: KeyboardEvent) => {
              if (event.key === 'Enter') {
                commitPercentValue((event.target as HTMLInputElement).value);
                (event.target as HTMLInputElement).blur();
              }
            }}
            aria-label={`${ariaLabel || 'Slider value'} percent`}
            style={{ ...inputBaseStyle, width: '72px', textAlign: 'right' }}
          />
          <span style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textSecondary }}>%</span>
        </div>
      </div>
    </>
  );
}
