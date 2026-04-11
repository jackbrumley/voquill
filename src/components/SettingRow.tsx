import { ComponentChildren } from 'preact';

interface SettingRowProps {
  title: string;
  description?: string;
  status?: ComponentChildren;
  children?: ComponentChildren;
  className?: string;
}

export const SettingRow = ({
  title,
  description,
  status,
  children,
  className = '',
}: SettingRowProps) => {
  return (
    <div className={`setting-row ${className}`.trim()}>
      <div className="setting-row-header">
        <div className="field-label">{title}</div>
        {status ? <div className="setting-row-status">{status}</div> : null}
      </div>
      {description ? <p className="field-description">{description}</p> : null}
      {children != null ? <div className="field-content">{children}</div> : null}
    </div>
  );
};
