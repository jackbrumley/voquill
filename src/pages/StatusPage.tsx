import { IconBrandGithub, IconHeart } from '@tabler/icons-preact';
import { open } from '@tauri-apps/plugin-shell';
import StatusIcon from '../StatusIcon.tsx';
import { Card } from '../components/Card.tsx';
import { Button } from '../components/Button.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';

interface StatusPageProps {
  currentStatus: string;
  appVersion: string;
  modelStatus: Record<string, boolean>;
  isSystemManagedShortcut: boolean;
  config: {
    transcription_mode: 'API' | 'Local';
    output_method: 'Typewriter' | 'Clipboard';
    local_model_size: string;
    hotkey: string;
  };
  onToggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
}

export function StatusPage({
  currentStatus,
  appVersion,
  modelStatus,
  isSystemManagedShortcut,
  config,
  onToggleOutputMethod,
}: StatusPageProps) {
  return (
    <div className="tab-panel page-scroll" key="status">
      <div className="tab-panel-padded">
        <div className="status-display">
          <StatusIcon status={currentStatus} large />
          <div className="status-text-app" key={`text-${currentStatus}`}>
            {currentStatus === 'Transcribing' ? `Transcribing (${config.transcription_mode})` : currentStatus}
          </div>
          <div className="mode-selection-group">
            <ModeSwitcher
              value={config.output_method}
              onToggle={onToggleOutputMethod}
              options={[
                { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter Mode: Simulates key presses' },
                { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard Mode: Fast copy-paste' },
              ]}
            />
            <div className="mode-description" key={`desc-${config.output_method}`}>
              {config.output_method === 'Typewriter'
                ? 'Types directly into your active cursor.'
                : 'Copies results to your clipboard.'}
            </div>
          </div>
        </div>

        <Card className="help-content">
          <h3>How to Use Voquill</h3>
          <ol className="instructions">
            {config.transcription_mode === 'Local' ? (
              modelStatus[config.local_model_size] ? (
                <li>Local Whisper model is <strong>Ready</strong>.</li>
              ) : (
                <li>Download a <strong>Whisper model</strong> in Config.</li>
              )
            ) : (
              <li>Enter your <strong>OpenAI API key</strong> in Config.</li>
            )}
            <li>Position cursor in any text field.</li>
            <li>
              {isSystemManagedShortcut
                ? 'Hold your system shortcut and speak.'
                : <><span>Hold </span><strong>{config.hotkey}</strong><span> and speak.</span></>}
            </li>
            <li>Release keys to transcribe and type.</li>
          </ol>
        </Card>

        <div className="status-footer">
          <div className="status-footer-links">
            <Button variant="icon" className="github-link" onClick={() => open('https://github.com/jackbrumley/voquill')} title="GitHub Repository">
              <IconBrandGithub size={18} />
            </Button>
            <Button variant="icon" className="github-link" onClick={() => open('https://voquill.org/donate')} title="Support the project">
              <IconHeart size={18} color="#ff6b6b" fill="#ff6b6b" fillOpacity={0.2} />
            </Button>
          </div>
          <div className="version-text">v{appVersion}</div>
        </div>
      </div>
    </div>
  );
}
