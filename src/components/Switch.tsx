
import { invoke } from '@tauri-apps/api/core';

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  className?: string;
}

export const Switch = ({ checked, onChange, label, className = '' }: SwitchProps) => {
  return (
    <label className={`switch-container ${className}`}>
      {label && <span className="switch-label">{label}</span>}
      <div className="switch-wrapper">
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
        />
        <span className="switch-slider"></span>
      </div>
    </label>
  );
};
