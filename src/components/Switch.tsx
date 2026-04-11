
import { invoke } from '@tauri-apps/api/core';
import { tokens } from '../design-tokens.ts';

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  className?: string;
}

export const Switch = ({ checked, onChange, label, className = '' }: SwitchProps) => {
  return (
    <label
      className={className}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: tokens.spacing.md,
        width: '100%',
        cursor: 'pointer',
      }}
    >
      {label && <span style={{ fontWeight: 600, color: tokens.colors.textPrimary, fontSize: tokens.typography.sizeSm }}>{label}</span>}
      <div style={{ position: 'relative', width: '40px', height: '22px' }}>
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => {
            const nextValue = (e.target as HTMLInputElement).checked;
            const switchLabel = label || 'Unnamed switch';
            invoke('log_ui_event', {
              message: `🖱️ Switch toggled: ${switchLabel} -> ${nextValue ? 'On' : 'Off'}`,
            }).catch(() => {});
            onChange(nextValue);
          }}
          style={{ position: 'absolute', opacity: 0, width: 0, height: 0 }}
        />
        <span
          style={{
            position: 'absolute',
            inset: 0,
            background: checked ? tokens.colors.accentPrimary : 'rgba(255, 255, 255, 0.2)',
            borderRadius: '999px',
            transition: 'all 0.2s ease',
          }}
        >
          <span
            style={{
              position: 'absolute',
              top: '2px',
              left: checked ? '20px' : '2px',
              width: '18px',
              height: '18px',
              background: '#fff',
              borderRadius: '999px',
              transition: 'left 0.2s ease',
            }}
          ></span>
        </span>
      </div>
    </label>
  );
};
