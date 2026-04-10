import { IconInfoCircle } from '@tabler/icons-preact';
import { Button } from './Button.tsx';

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
}: ModelSelectionPanelProps) {
  return (
    <>
      <div className="select-wrapper">
        {availableModels.length > 0 ? (
          <>
            <select value={localModelSize} onChange={(event: Event) => onChangeModel((event.target as HTMLSelectElement).value)}>
              {availableModels
                .filter((model) => model.engine === localEngine)
                .map((model) => (
                  <option key={model.size} value={model.size}>
                    {model.label} {model.recommended ? '(Recommended)' : ''} ({Math.round(model.file_size / 1024 / 1024)}MB)
                  </option>
                ))}
            </select>
            <Button variant="ghost" className="icon-button" onClick={onShowModelGuide} title="Model Guide">
              <IconInfoCircle size={20} />
            </Button>
            {!modelStatus[localModelSize] && (
              <Button size="sm" onClick={() => onDownloadModel(localModelSize)} disabled={isDownloading}>
                {isDownloading ? '...' : 'Download'}
              </Button>
            )}
          </>
        ) : (
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center', width: '100%' }}>
            <div style={{ fontSize: '12px', color: 'var(--color-text-dim)', flex: 1 }}>Loading models...</div>
            <Button size="sm" onClick={onRetryModels}>Retry</Button>
          </div>
        )}
      </div>

      {availableModels.length > 0 && (
        <div className="mode-description" style={{ textAlign: 'left', marginTop: '4px' }}>
          {availableModels.find((model) => model.size === localModelSize)?.description}
        </div>
      )}

      {isDownloading && (
        <div className="download-progress-container" style={{ marginTop: '-8px', marginBottom: '16px' }}>
          <div className="volume-meter-container" style={{ height: '4px' }}>
            <div className="volume-meter-bar" style={{ width: `${Math.min(downloadProgress, 100)}%`, background: 'var(--colors-accent-primary)' }}></div>
          </div>
          <div style={{ fontSize: '10px', color: 'var(--color-text-dim)', textAlign: 'right', marginTop: '2px' }}>
            Downloading model... {Math.round(downloadProgress)}%
          </div>
        </div>
      )}
    </>
  );
}
