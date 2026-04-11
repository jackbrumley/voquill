import { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import {
  getSettingRowStyle,
  settingRowContentStyle,
  settingRowDescriptionStyle,
  settingRowHeaderStyle,
  settingRowLabelStyle,
  settingRowStatusStyle,
} from '../theme/component-styles.ts';

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
  const [focused, setFocused] = useState(false);
  const isReady = className.split(/\s+/).includes('ready');

  return (
    <div
      className={`setting-row ${className}`.trim()}
      style={getSettingRowStyle({ focused, ready: isReady })}
      onFocusCapture={() => setFocused(true)}
      onBlurCapture={() => setFocused(false)}
    >
      <div className="setting-row-header" style={settingRowHeaderStyle}>
        <div className="field-label" style={settingRowLabelStyle}>{title}</div>
        {status ? <div className="setting-row-status" style={settingRowStatusStyle}>{status}</div> : null}
      </div>
      {description ? <p className="field-description" style={settingRowDescriptionStyle}>{description}</p> : null}
      {children != null ? <div className="field-content" style={settingRowContentStyle}>{children}</div> : null}
    </div>
  );
};
