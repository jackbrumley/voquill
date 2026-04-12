import { IconInfoCircle } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { SelectField } from './SelectField.tsx';
import { selectWrapperStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

interface ModelSelectionPanelProps {
  availableModels: any[];
  localEngine: string;
  localModelSize: string;
  modelStatus: Record<string, boolean>;
  isDownloading: boolean;
  downloadProgress: number;
  onChangeModel: (size: string) => void;
  onShowModelGuide: () => void;
  onDownloadModel: (size: string) => void;
  onRetryModels: () => void;
  actionButtonSize?: 'sm' | 'md' | 'lg';
}

export function ModelSelectionPanel({
  availableModels,
  localEngine,
  localModelSize,
  modelStatus,
  isDownloading,
  downloadProgress,
  onChangeModel,
  onShowModelGuide,
  onDownloadModel,
  onRetryModels,
  actionButtonSize = 'md',
}: ModelSelectionPanelProps) {
  const actionButtonRowStyle = {
    display: 'flex',
    justifyContent: 'center',
    width: '100%',
  } as const;

  return (
    <>
      <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
        {availableModels.length > 0 ? (
          <>
            <div style={selectWrapperStyle}>
              <div style={{ flex: 1, minWidth: 0 }}>
                <SelectField
                  value={localModelSize}
                  onChange={onChangeModel}
                  searchable={false}
                  ariaLabel="Local model"
                  options={availableModels
                    .filter((model) => model.engine === localEngine)
                    .map((model) => ({
                      value: model.size,
                      label: `${model.label} ${model.recommended ? '(Recommended) ' : ''}(${Math.round(model.file_size / 1024 / 1024)}MB)`,
                      searchText: `${model.size} ${model.description || ''} ${model.engine || ''}`,
                    }))}
                  style={{ minWidth: 0 }}
                />
              </div>
              <Button variant="icon" onClick={onShowModelGuide} title="Model Guide">
                <IconInfoCircle size={20} />
              </Button>
            </div>
            {!modelStatus[localModelSize] && (
              <div style={actionButtonRowStyle}>
                <Button variant="configAction" size={actionButtonSize} onClick={() => onDownloadModel(localModelSize)} disabled={isDownloading}>
                  {isDownloading ? '...' : 'Download'}
                </Button>
              </div>
            )}
          </>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.sm, width: '100%' }}>
              <div style={{ fontSize: '12px', color: '#d9dfe7', flex: 1, minWidth: 0 }}>Loading models...</div>
              <Button variant="icon" onClick={onShowModelGuide} title="Model Guide">
                <IconInfoCircle size={20} />
              </Button>
            </div>
            <div style={actionButtonRowStyle}>
              <Button variant="configAction" size={actionButtonSize} onClick={onRetryModels}>Retry</Button>
            </div>
          </div>
        )}
      </div>

      {isDownloading && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', width: '100%' }}>
          <div style={{ width: '100%', height: '4px', background: tokens.colors.bgTertiary, borderRadius: '2px', overflow: 'hidden' }}>
            <div style={{ width: `${Math.min(downloadProgress, 100)}%`, height: '100%', background: tokens.colors.success }}></div>
          </div>
          <div style={{ fontSize: '10px', color: '#d9dfe7', textAlign: 'right' }}>
            Downloading model... {Math.round(downloadProgress)}%
          </div>
        </div>
      )}

      {availableModels.length > 0 && (
        <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textSecondary }}>
          {availableModels.find((model) => model.size === localModelSize)?.description}
        </div>
      )}
    </>
  );
}
