import { inputBaseStyle } from '../theme/ui-primitives.ts';

interface NumberFieldProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
}

export function NumberField({ value, onChange, min, max, step = 1 }: NumberFieldProps) {
  const handleChange = (event: Event) => {
    const rawValue = (event.target as HTMLInputElement).value;
    const nextValue = Number(rawValue);
    if (Number.isNaN(nextValue)) {
      return;
    }
    onChange(nextValue);
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
        value={value}
        onChange={handleChange}
        min={min}
        max={max}
        step={step}
        style={inputBaseStyle}
      />
    </>
  );
}
