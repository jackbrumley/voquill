import type { JSX } from 'preact';
import { useSignal, useSignalEffect } from '@preact/signals';
import { tokens } from '../design-tokens.ts';

let nextSliderId = 0;

interface SliderFieldProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  ariaLabel?: string;
  style?: JSX.CSSProperties;
}

export function SliderField({
  value,
  min = 0,
  max = 1,
  onChange,
  ariaLabel,
  style,
}: SliderFieldProps) {
  const minPercent = Math.round(min * 100);
  const maxPercent = Math.round(max * 100);
  const currentPercent = Math.round(value * 100);
  const draftPercent = useSignal(String(currentPercent));
  const id = useSignal<string>(`slider-${++nextSliderId}`);

  useSignalEffect(() => {
    draftPercent.value = String(currentPercent);
  });

  const clampPercent = (nextPercent: number) => Math.min(maxPercent, Math.max(minPercent, nextPercent));

  const commitPercentValue = (rawValue: string) => {
    if (rawValue.trim() === '') {
      draftPercent.value = String(currentPercent);
      return;
    }

    const parsedPercent = Number(rawValue);
    if (Number.isNaN(parsedPercent)) {
      draftPercent.value = String(currentPercent);
      return;
    }

    const clampedPercent = clampPercent(Math.round(parsedPercent));
    draftPercent.value = String(clampedPercent);
    onChange(clampedPercent / 100);
  };

  const handleInput = (event: Event) => {
    const rawValue = (event.target as HTMLInputElement).value;
    draftPercent.value = rawValue;
    commitPercentValue(rawValue);
  };

  const fillPercent = ((value - min) / (max - min)) * 100;

  return (
    <>
      <style>{`
.${id.value}::-webkit-slider-runnable-track {
  width: 100%;
  height: 6px;
  background: linear-gradient(to right, ${tokens.colors.accentPrimary} 0%, ${tokens.colors.accentPrimary} ${fillPercent}%, rgba(255,255,255,0.12) ${fillPercent}%, rgba(255,255,255,0.12) 100%);
  border-radius: 3px;
}
.${id.value}::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  background: #fff;
  border-radius: 50%;
  cursor: pointer;
  margin-top: -5px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.3);
}
.${id.value}::-moz-range-track {
  width: 100%;
  height: 6px;
  background: linear-gradient(to right, ${tokens.colors.accentPrimary} 0%, ${tokens.colors.accentPrimary} ${fillPercent}%, rgba(255,255,255,0.12) ${fillPercent}%, rgba(255,255,255,0.12) 100%);
  border-radius: 3px;
}
.${id.value}::-moz-range-thumb {
  width: 16px;
  height: 16px;
  background: #fff;
  border-radius: 50%;
  cursor: pointer;
  border: none;
  box-shadow: 0 1px 3px rgba(0,0,0,0.3);
}
.${id.value} {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  height: 20px;
  background: transparent;
  outline: none;
  cursor: pointer;
}
      `}</style>
      <div style={{ width: '100%' }}>
        <input
          type="range"
          className={id.value}
          value={draftPercent.value}
          onInput={handleInput}
          aria-label={ariaLabel}
          step={1}
          min={minPercent}
          max={maxPercent}
          style={style as Record<string, string | number> | undefined}
        />
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, marginTop: '-4px' }}>
          <span>{minPercent}%</span>
          <span>{maxPercent}%</span>
        </div>
      </div>
    </>
  );
}