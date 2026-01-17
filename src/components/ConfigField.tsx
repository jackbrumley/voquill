
import { ComponentChildren } from 'preact';

interface ConfigFieldProps {
  label: string;
  description?: string;
  children: ComponentChildren;
  className?: string;
}

export const ConfigField = ({ label, description, children, className = '' }: ConfigFieldProps) => {
  return (
    <div className={`config-field ${className}`}>
      <label className="field-label">{label}</label>
      {description && <p className="field-description">{description}</p>}
      <div className="field-content">
        {children}
      </div>
    </div>
  );
};
