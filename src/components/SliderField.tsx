import type { JSX } from 'preact';
import { useSignal, useSignalEffect } from '@preact/signals';
import { inputBaseStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

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

  return (
    <input
      type="range"
      value={draftPercent.value}
      onInput={handleInput}
      aria-label={ariaLabel}
      step={1}
      min={minPercent}
      max={maxPercent}
      style={{ ...inputBaseStyle, accentColor: tokens.colors.accentPrimary, ...style } as Record<string, string | number>}
    />
  );
}