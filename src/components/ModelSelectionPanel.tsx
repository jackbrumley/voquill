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
              <Button variant="configAction" size={actionButtonSize} onClick={() => onDownloadModel(localModelSize)} disabled={isDownloading}>
                {isDownloading ? '...' : 'Download'}
              </Button>
            )}
          </>
        ) : (
          <div className="model-panel-row">
            <div className="model-panel-loading-text">Loading models...</div>
            <Button variant="configAction" size={actionButtonSize} onClick={onRetryModels}>Retry</Button>
          </div>
        )}
      </div>

      {availableModels.length > 0 && (
        <div className="mode-description model-panel-description">
          {availableModels.find((model) => model.size === localModelSize)?.description}
        </div>
      )}

      {isDownloading && (
        <div className="download-progress-container model-download-progress">
          <div className="volume-meter-container model-download-progress-bar">
            <div className="volume-meter-bar model-download-progress-fill" style={{ width: `${Math.min(downloadProgress, 100)}%` }}></div>
          </div>
          <div className="model-download-progress-text">
            Downloading model... {Math.round(downloadProgress)}%
          </div>
        </div>
      )}
    </>
  );
}
