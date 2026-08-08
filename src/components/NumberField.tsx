import { useSignal, useSignalEffect } from '@preact/signals';
import { inputBaseStyle } from '../theme/ui-primitives.ts';

interface NumberFieldProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
}

export function NumberField({ value, onChange, min, max, step = 1, disabled = false }: NumberFieldProps) {
  const draftValue = useSignal(String(value));
  const isFocused = useSignal(false);

  useSignalEffect(() => {
    if (!isFocused.value) {
      draftValue.value = String(value);
    }
  });

  const commitIfValid = (rawValue: string) => {
    if (rawValue.trim() === '') {
      return;
    }
    const nextValue = Number(rawValue);
    if (Number.isNaN(nextValue)) {
      return;
    }
    onChange(nextValue);
  };

  const handleInput = (event: Event) => {
    if (disabled) {
      return;
    }
    const rawValue = (event.target as HTMLInputElement).value;
    draftValue.value = rawValue;
    commitIfValid(rawValue);
  };

  const handleBlur = () => {
    if (disabled) {
      return;
    }
    isFocused.value = false;
    if (draftValue.value.trim() === '' || Number.isNaN(Number(draftValue.value))) {
      draftValue.value = String(value);
      return;
    }
    commitIfValid(draftValue.value);
  };

  return (
    <>
      <style>{`
        .voquill-number-field {
          -moz-appearance: textfield;
          appearance: textfield;
        }
        .voquill-number-field::-webkit-outer-spin-button,
        .voquill-number-field::-webkit-inner-spin-button {
          -webkit-appearance: none;
          margin: 0;
        }
      `}</style>
      <input
        className="voquill-number-field"
        type="number"
        value={draftValue.value}
        onInput={handleInput}
        onFocus={() => { isFocused.value = true; }}
        onBlur={handleBlur}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            (event.target as HTMLInputElement).blur();
          }
        }}
        min={min}
        max={max}
        step={step}
        disabled={disabled}
        style={inputBaseStyle}
      />
    </>
  );
}