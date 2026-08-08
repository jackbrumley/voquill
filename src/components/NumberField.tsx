import { useSignal, useSignalEffect } from '@preact/signals';
import { tokens } from '../design-tokens.ts';

interface NumberFieldProps {
  value: number;
  min: number;
  max?: number;
  step?: number;
  onChange: (value: number) => void;
  label?: string;
  disabled?: boolean;
}

export function NumberField({ value, min, max = 100, step = 1, onChange, label, disabled }: NumberFieldProps) {
  const draftValue = useSignal(String(value));
  const isFocused = useSignal(false);

  useSignalEffect(() => {
    if (!isFocused.value) {
      draftValue.value = String(value);
    }
  });

  const handleChange = (e: Event) => {
    const target = e.currentTarget as HTMLInputElement;
    draftValue.value = target.value;
    const parsed = parseFloat(target.value);
    if (!isNaN(parsed)) {
      onChange(Math.min(max, Math.max(min, parsed)));
    }
  };

  const handleBlur = () => {
    isFocused.value = false;
    draftValue.value = String(value);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
      {label && <label style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textSecondary }}>{label}</label>}
      <input
        type="number"
        value={draftValue.value}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        onInput={handleChange}
        onFocus={() => { isFocused.value = true; }}
        onBlur={handleBlur}
        style={{
          background: tokens.colors.bgTertiary,
          border: `1px solid rgba(255, 255, 255, 0.1)`,
          borderRadius: tokens.radii.input,
          color: tokens.colors.textPrimary,
          fontSize: tokens.typography.sizeSm,
          padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`,
          outline: 'none',
          width: '100%',
          boxSizing: 'border-box',
        }}
      />
    </div>
  );
}