import { invoke } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { Button } from '../../components/Button.tsx';
import { NumberField } from '../../components/NumberField.tsx';
import { SelectField } from '../../components/SelectField.tsx';
import type { Config } from '../../types.ts';
import { useDiagnostics } from '../../hooks/useDiagnostics.ts';
import { selectWrapperStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';

const configGhostPillStyle = {
  background: 'rgba(255, 255, 255, 0.05)',
  border: '1px solid rgba(255, 255, 255, 0.1)',
  color: tokens.colors.textPrimary,
  padding: '6px 14px',
} as const;

const systemInfoListStyle = {
  display: 'grid',
  gridTemplateColumns: 'auto 1fr',
  columnGap: tokens.spacing.sm,
  rowGap: '2px',
  margin: 0,
  fontSize: tokens.typography.sizeXs,
} as const;

const systemInfoTermStyle = {
  color: tokens.colors.textMuted,
  fontSize: tokens.typography.sizeXs,
} as const;

const systemInfoValueStyle = {
  margin: 0,
  color: tokens.colors.textPrimary,
  fontFamily: tokens.typography.fontMono,
  overflowWrap: 'anywhere',
} as const;

interface DebugSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
  onReopenInitialSetup: () => void;
  onFactoryReset: () => void;
  onOpenUiLab: () => void;
  showToast?: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

export function DebugSection({
  config,
  updateConfig,
  onReopenInitialSetup,
  onFactoryReset,
  onOpenUiLab,
  showToast,
}: DebugSectionProps) {
  const diagnostics = useDiagnostics(showToast);
  const systemInfo = diagnostics.systemInfo;

  return (
    <>
      <ConfigField label="Logs" description="For a bug report, attach the copied log and system info.">
        <div style={{ display: 'flex', gap: tokens.spacing.xs, flexWrap: 'wrap' }}>
          <Button variant="configAction" onClick={diagnostics.copyLog}>Copy Log</Button>
          <Button variant="ghost" pill style={configGhostPillStyle} onClick={diagnostics.openLog}>Open Log</Button>
          <Button variant="ghost" pill style={configGhostPillStyle} onClick={diagnostics.openLogFolder}>Open Log Folder</Button>
        </div>
      </ConfigField>

      <ConfigField label="System Info" description="The details a bug report needs about this machine.">
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
          {systemInfo ? (
            <dl style={systemInfoListStyle}>
              <dt style={systemInfoTermStyle}>Version</dt>
              <dd style={systemInfoValueStyle}>Voquill v{systemInfo.app_version} ({systemInfo.build_profile})</dd>
              <dt style={systemInfoTermStyle}>OS</dt>
              <dd style={systemInfoValueStyle}>{systemInfo.distribution} ({systemInfo.os})</dd>
              <dt style={systemInfoTermStyle}>Architecture</dt>
              <dd style={systemInfoValueStyle}>{systemInfo.architecture}</dd>
              <dt style={systemInfoTermStyle}>Desktop</dt>
              <dd style={systemInfoValueStyle}>{systemInfo.desktop} ({systemInfo.session_type})</dd>
            </dl>
          ) : (
            <span style={systemInfoTermStyle}>System info unavailable.</span>
          )}
          <div>
            <Button variant="ghost" pill style={configGhostPillStyle} onClick={diagnostics.copySystemInfo}>Copy System Info</Button>
          </div>
        </div>
      </ConfigField>

      <ConfigField label="Session Logging Level" description="Control how detailed log entries are in the diagnostic log.">
        <div style={selectWrapperStyle}>
          <SelectField
            value={config.log_level || 'info'}
            options={[
              { value: 'error', label: 'Error' },
              { value: 'warn', label: 'Warn' },
              { value: 'info', label: 'Info' },
              { value: 'debug', label: 'Debug' },
              { value: 'trace', label: 'Trace' },
            ]}
            onChange={(level) => updateConfig('log_level', level)}
            ariaLabel="Log level"
          />
        </div>
      </ConfigField>

      <ConfigField label="Save Recording Logs" description="Save raw audio from recordings and file imports as WAV files in the debug folder for troubleshooting.">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
          <Switch name="Save Recording Logs" checked={config.enable_recording_logs} onChange={(checked) => updateConfig('enable_recording_logs', checked)} />
          <div style={{ display: 'flex', gap: tokens.spacing.xs }}>
            <Button variant="ghost" pill style={configGhostPillStyle} onClick={diagnostics.openLogFolder}>Open Folder</Button>
            <Button
              variant="danger"
              pill
              style={configGhostPillStyle}
              onClick={async () => {
                if (await confirm('Delete all recorded WAV files?')) {
                  try {
                    const count = await invoke<number>('clear_recording_logs');
                    showToast?.(`Deleted ${count} recording file(s)`, 'success');
                  } catch (e) {
                    console.error('Failed to delete recordings:', e);
                    showToast?.('Failed to delete recordings', 'error');
                  }
                }
              }}
            >
              Delete
            </Button>
          </div>
        </div>
      </ConfigField>

      <ConfigField label="History Limit" description="Maximum number of history entries to keep. Oldest entries are automatically pruned when the limit is exceeded. 0 = unlimited.">
        <NumberField value={config.history_limit} onChange={(value) => updateConfig('history_limit', value)} min={0} max={10000} />
      </ConfigField>

      <ConfigField label="Unload Model" description="Free GPU/CPU memory by unloading the transcription model from memory. It will reload automatically on next dictation.">
        <Button variant="configAction" onClick={async () => { try { await invoke('unload_model'); } catch (e) { console.error('Failed to unload model:', e); } }}>Unload Model</Button>
      </ConfigField>

      <ConfigField label="Initial Setup" description="Re-open onboarding checks for permissions, model, and hotkey setup.">
        <Button variant="configAction" onClick={onReopenInitialSetup}>Re-run Initial Setup</Button>
      </ConfigField>

      <ConfigField label="Factory Reset" description="Reset Voquill to defaults and clear models, logs, and history.">
        <Button variant="danger" pill onClick={onFactoryReset}>Reset App to Defaults</Button>
      </ConfigField>

      <ConfigField label="UI Lab" labelBadge="Experimental" description="Open the internal visual QA page for component and state previews.">
        <Button variant="ghost" pill style={configGhostPillStyle} onClick={onOpenUiLab}>Open UI Lab</Button>
      </ConfigField>
    </>
  );
}
