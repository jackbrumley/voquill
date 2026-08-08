import { useSignal, useSignalEffect } from '@preact/signals';
import { tokens } from '../design-tokens.ts';

interface SliderFieldProps {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  label?: string;
  displayValue?: string;
  ariaLabel?: string;
  style?: Record<string, string | number>;
}

export function SliderField({ value, min, max, step, onChange, label, displayValue, ariaLabel, style }: SliderFieldProps) {
  const draftPercent = useSignal(String(((value - min) / (max - min)) * 100));

  useSignalEffect(() => {
    draftPercent.value = String(((value - min) / (max - min)) * 100);
  });

  const handleChange = (e: Event) => {
    const target = e.currentTarget as HTMLInputElement;
    const percent = parseFloat(target.value);
    draftPercent.value = String(percent);
    const actualValue = min + (percent / 100) * (max - min);
    const stepped = Math.round(actualValue / step) * step;
    onChange(Math.min(max, Math.max(min, stepped)));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', ...style }}>
      {label && (
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <label style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textSecondary }}>{label}</label>
          <span style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>{displayValue ?? value}</span>
        </div>
      )}
      <input
        type="range"
        min={0}
        max={100}
        step={0.1}
        value={draftPercent.value}
        aria-label={ariaLabel}
        onInput={handleChange}
        style={{ width: '100%', accentColor: tokens.colors.accentPrimary }}
      />
    </div>
  );
}