import { ComponentChildren } from 'preact';
import {
  getSettingRowStyle,
  settingRowContentStyle,
  settingRowDescriptionStyle,
  settingRowHeaderStyle,
  settingRowHeaderRightStyle,
  settingRowLabelBadgeStyle,
  settingRowLabelStyle,
  settingRowStatusStyle,
} from '../theme/component-styles.ts';

interface SettingRowProps {
  title: string;
  titleBadge?: string;
  description?: string;
  status?: ComponentChildren;
  children?: ComponentChildren;
  className?: string;
}

export const SettingRow = ({
  title,
  titleBadge,
  description,
  status,
  children,
  className = '',
}: SettingRowProps) => {
  const isReady = className.split(/\s+/).includes('ready');

  return (
    <div
      className={`setting-row ${className}`.trim()}
      style={getSettingRowStyle({ ready: isReady })}
    >
      <div className="setting-row-header" style={settingRowHeaderStyle}>
        <div className="field-label" style={settingRowLabelStyle}>{title}</div>
        {(titleBadge || status) ? (
          <div className="setting-row-right" style={settingRowHeaderRightStyle}>
            {titleBadge ? <span style={settingRowLabelBadgeStyle}>{titleBadge}</span> : null}
            {status ? <div className="setting-row-status" style={settingRowStatusStyle}>{status}</div> : null}
          </div>
        ) : null}
      </div>
      {description ? <p className="field-description" style={settingRowDescriptionStyle}>{description}</p> : null}
      {children != null ? <div className="field-content" style={settingRowContentStyle}>{children}</div> : null}
    </div>
  );
};
