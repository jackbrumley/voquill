import { NumberField } from './NumberField.tsx';
import { SelectField } from './SelectField.tsx';
import { Switch } from './Switch.tsx';
import { ConfigField } from './ConfigField.tsx';
import { tokens } from '../design-tokens.ts';
import type { EngineCapabilities } from '../types.ts';

interface EngineSettingsFieldsProps {
  capabilities: EngineCapabilities;
  values: Record<string, unknown>;
  onChange: (key: string, value: unknown) => void;
}

/// One settings row per setting the selected engine reports, laid out like
/// every other row on the page.
export function EngineSettingsFields({ capabilities, values, onChange }: EngineSettingsFieldsProps) {
  return (
    <>
      {capabilities.settings.map((setting) => (
        <ConfigField key={setting.key} label={setting.label} description={setting.description}>
          {renderSettingControl(setting, values[setting.key] ?? setting.default, onChange)}
        </ConfigField>
      ))}
    </>
  );
}

function renderSettingControl(
  setting: EngineCapabilities['settings'][0],
  currentValue: unknown,
  onChange: (key: string, value: unknown) => void,
) {
  switch (setting.settingType) {
    case 'number':
      return (
        <NumberField
          value={Number(currentValue) || 0}
          onChange={(val) => onChange(setting.key, val)}
          min={1}
          max={64}
        />
      );
    case 'bool':
      return (
        <Switch
          name={setting.label}
          checked={Boolean(currentValue)}
          onChange={(checked) => onChange(setting.key, checked)}
        />
      );
    case 'select':
      return (
        <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.sm, width: '100%' }}>
          <SelectField
            value={String(currentValue)}
            options={(setting.options || []).map((opt) => ({ value: opt.value, label: opt.label }))}
            onChange={(val) => onChange(setting.key, val)}
            ariaLabel={setting.label}
          />
        </div>
      );
    default:
      return null;
  }
}