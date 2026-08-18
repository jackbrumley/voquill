import { Card } from '../components/Card.tsx';
import { tabPanelPaddedStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

interface HelpPageProps {
  config: {
    transcription_mode: 'API' | 'Local';
    local_model_size: string;
    hotkey: string;
    hotkey_mode: 'HoldToTalk' | 'Toggle';
  };
  modelStatus: Record<string, boolean>;
  isSystemManagedShortcut: boolean;
}

export function HelpPage({ config, modelStatus, isSystemManagedShortcut }: HelpPageProps) {
  const isToggleMode = config.hotkey_mode === 'Toggle';

  const howToSteps = [
    config.transcription_mode === 'Local'
      ? (modelStatus[config.local_model_size]
        ? <>Local Whisper model is <strong style={{ color: '#ffffff' }}>Ready</strong>.</>
        : <>Download a <strong style={{ color: '#ffffff' }}>Whisper model</strong> in Settings.</>)
      : <>Enter your <strong style={{ color: '#ffffff' }}>OpenAI API key</strong> in Settings.</>,
    <>Position cursor in any text field.</>,
    isToggleMode
      ? (isSystemManagedShortcut
        ? <>Press your system shortcut to start recording.</>
        : <><span>Press </span><strong style={{ color: '#818cf8' }}>{config.hotkey}</strong><span> to start recording.</span></>)
      : (isSystemManagedShortcut
        ? <>Hold your system shortcut and speak.</>
        : <><span>Hold </span><strong style={{ color: '#818cf8' }}>{config.hotkey}</strong><span> and speak.</span></>),
    isToggleMode
      ? <>Press it again to stop and transcribe.</>
      : <>Release keys to transcribe and type.</>,
  ];

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto' }} key="help">
      <div style={{ ...tabPanelPaddedStyle, flex: 1 }}>
        <div style={{
          fontSize: '11px',
          fontWeight: 600,
          color: '#cbd5e1',
          letterSpacing: '0.06em',
          textTransform: 'uppercase',
          marginBottom: '-6px',
          paddingLeft: '2px',
        }}>
          How to Use Voquill
        </div>

        <Card>
          <ol style={{ listStyle: 'none', margin: 0, padding: 0, textAlign: 'left', fontSize: tokens.typography.sizeSm }}>
            {howToSteps.map((step, index) => (
              <li key={index} style={{ display: 'grid', gridTemplateColumns: '24px 1fr', alignItems: 'start', marginBottom: '8px', color: '#e2e8f0', lineHeight: 1.5 }}>
                <span style={{ color: '#818cf8', fontWeight: 800, fontFamily: tokens.typography.fontMono, fontSize: tokens.typography.sizeSm }}>
                  {index + 1}.
                </span>
                <span>{step}</span>
              </li>
            ))}
          </ol>
        </Card>

        <div style={{
          fontSize: '11px',
          fontWeight: 600,
          color: '#cbd5e1',
          letterSpacing: '0.06em',
          textTransform: 'uppercase',
          marginBottom: '-6px',
          paddingLeft: '2px',
          marginTop: '8px',
        }}>
          Tips
        </div>

        <Card>
          <ul style={{ listStyle: 'none', margin: 0, padding: 0, textAlign: 'left', fontSize: tokens.typography.sizeSm }}>
            <li style={{ display: 'grid', gridTemplateColumns: '20px 1fr', alignItems: 'start', marginBottom: '8px', color: '#e2e8f0', lineHeight: 1.5 }}>
              <span style={{ color: '#818cf8', fontWeight: 800, fontFamily: tokens.typography.fontMono, fontSize: tokens.typography.sizeSm }}>
                •
              </span>
              <span>Hold to talk pauses recording when you stop speaking; toggle mode records until you press the hotkey again.</span>
            </li>
            <li style={{ display: 'grid', gridTemplateColumns: '20px 1fr', alignItems: 'start', marginBottom: '8px', color: '#e2e8f0', lineHeight: 1.5 }}>
              <span style={{ color: '#818cf8', fontWeight: 800, fontFamily: tokens.typography.fontMono, fontSize: tokens.typography.sizeSm }}>
                •
              </span>
              <span>Pressing the hotkey while transcribing cancels the transcription.</span>
            </li>
            <li style={{ display: 'grid', gridTemplateColumns: '20px 1fr', alignItems: 'start', color: '#e2e8f0', lineHeight: 1.5 }}>
              <span style={{ color: '#818cf8', fontWeight: 800, fontFamily: tokens.typography.fontMono, fontSize: tokens.typography.sizeSm }}>
                •
              </span>
              <span>Enable "Differentiate voices" on the file upload or in Settings under Transcription to label speakers in imported files. Enable it in Settings under Transcription for in-app recordings (may be slower).</span>
            </li>
          </ul>
        </Card>
      </div>
    </div>
  );
}
