
import { ComponentChildren } from 'preact';

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
          onChange={(e) => onChange((e.target as HTMLInputElement).checked)} 
        />
        <span className="switch-slider"></span>
      </div>
    </label>
  );
};
