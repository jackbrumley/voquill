import { useEffect, useRef } from 'preact/hooks';
import { tokens } from '../../../design-tokens.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';

interface DurationInputProps {
  value: string;
  onChange: (val: string) => void;
  onSave: () => void;
  onCancel?: () => void;
}

export function DurationInput({ value, onChange, onSave, onCancel }: DurationInputProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const input = inputRef.current;
    if (input) {
      input.focus();
      input.select();
    }
  }, []);

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '3px' }}>
      <input
        ref={inputRef}
        type="text"
        inputMode="numeric"
        value={value}
        onInput={(e) => {
          const clean = (e.target as HTMLInputElement).value.replace(/[^0-9]/g, '');
          onChange(clean);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            onSave();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            if (onCancel) onCancel();
            else onSave();
          }
        }}
        onBlur={onSave}
        style={{
          ...inputBaseStyle,
          width: '50px',
          padding: '1px 4px',
          fontSize: '11px',
          fontFamily: 'monospace',
          textAlign: 'center',
        }}
      />
      <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>ms</span>
    </div>
  );
}
