import { NumberField } from './NumberField.tsx';
import { SelectField } from './SelectField.tsx';
import { Switch } from './Switch.tsx';
import { helperTextStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import type { EngineCapabilities } from '../types.ts';

interface EngineSettingsPanelProps {
  capabilities: EngineCapabilities;
  values: Record<string, unknown>;
  onChange: (key: string, value: unknown) => void;
}

export function EngineSettingsPanel({ capabilities, values, onChange }: EngineSettingsPanelProps) {
  if (capabilities.settings.length === 0) return null;

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      gap: tokens.spacing.sm,
      width: '100%',
      marginTop: tokens.spacing.xs,
    }}>
      <div style={{
        fontSize: '11px',
        fontWeight: 800,
        letterSpacing: '0.08em',
        textTransform: 'uppercase',
        color: tokens.colors.textSecondary,
        marginBottom: '2px',
      }}>
        Engine Settings
      </div>
      {capabilities.settings.map((setting) => {
        const currentValue = values[setting.key] ?? setting.default;
        const labelStyle = {
          fontSize: tokens.typography.sizeSm,
          fontWeight: 600,
          color: tokens.colors.textPrimary,
          marginBottom: '2px',
        };

        return (
          <div key={setting.key} style={{ width: '100%' }}>
            <div style={labelStyle}>{setting.label}</div>
            <div style={{ ...helperTextStyle, marginBottom: '4px' }}>{setting.description}</div>
            {renderSettingControl(setting, currentValue, onChange)}
          </div>
        );
      })}
    </div>
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