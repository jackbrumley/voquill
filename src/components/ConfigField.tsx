
import { ComponentChildren } from 'preact';
import { SettingRow } from './SettingRow.tsx';

interface ConfigFieldProps {
  label: string;
  labelBadge?: string;
  description?: string;
  children: ComponentChildren;
  className?: string;
}

export const ConfigField = ({ label, labelBadge, description, children, className = '' }: ConfigFieldProps) => {
  return (
    <SettingRow title={label} titleBadge={labelBadge} description={description} className={`config-field ${className}`.trim()}>
      {children}
    </SettingRow>
  );
};
