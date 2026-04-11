import { useEffect, useState } from 'preact/hooks';
import { inputBaseStyle } from '../theme/ui-primitives.ts';

interface NumberFieldProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
}

export function NumberField({ value, onChange, min, max, step = 1 }: NumberFieldProps) {
  const [draftValue, setDraftValue] = useState(String(value));
  const [isFocused, setIsFocused] = useState(false);

  useEffect(() => {
    if (!isFocused) {
      setDraftValue(String(value));
    }
  }, [value, isFocused]);

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
    const rawValue = (event.target as HTMLInputElement).value;
    setDraftValue(rawValue);
    commitIfValid(rawValue);
  };

  const handleBlur = () => {
    setIsFocused(false);
    if (draftValue.trim() === '' || Number.isNaN(Number(draftValue))) {
      setDraftValue(String(value));
      return;
    }
    commitIfValid(draftValue);
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
        value={draftValue}
        onInput={handleInput}
        onFocus={() => setIsFocused(true)}
        onBlur={handleBlur}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            (event.target as HTMLInputElement).blur();
          }
        }}
        min={min}
        max={max}
        step={step}
        style={inputBaseStyle}
      />
    </>
  );
}
