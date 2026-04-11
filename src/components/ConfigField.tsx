
import { ComponentChildren } from 'preact';
import { SettingRow } from './SettingRow.tsx';

interface ConfigFieldProps {
  label: string;
  description?: string;
  children: ComponentChildren;
  className?: string;
}

export const ConfigField = ({ label, description, children, className = '' }: ConfigFieldProps) => {
  return (
    <SettingRow title={label} description={description} className={`config-field ${className}`.trim()}>
      {children}
    </SettingRow>
  );
};
