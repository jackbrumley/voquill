import { IconAlertCircle, IconBrandGithub, IconHeart } from '@tabler/icons-preact';
import { open } from '@tauri-apps/plugin-shell';
import { useState } from 'preact/hooks';
import StatusIcon from '../StatusIcon.tsx';
import { Card } from '../components/Card.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { tabPanelPaddedStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

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
  hasUpdateAvailable: boolean;
  onOpenUpdateModal: () => void;
}

export function StatusPage({
  currentStatus,
  appVersion,
  modelStatus,
  isSystemManagedShortcut,
  config,
  onToggleOutputMethod,
  hasUpdateAvailable,
  onOpenUpdateModal,
}: StatusPageProps) {
  const [hoveredFooterIcon, setHoveredFooterIcon] = useState<'github' | 'heart' | null>(null);

  const howToSteps = [
    config.transcription_mode === 'Local'
      ? (modelStatus[config.local_model_size]
        ? <>Local Whisper model is <strong style={{ color: tokens.colors.textPrimary }}>Ready</strong>.</>
        : <>Download a <strong style={{ color: tokens.colors.textPrimary }}>Whisper model</strong> in Config.</>)
      : <>Enter your <strong style={{ color: tokens.colors.textPrimary }}>OpenAI API key</strong> in Config.</>,
    <>Position cursor in any text field.</>,
    isSystemManagedShortcut
      ? <>Hold your system shortcut and speak.</>
      : <><span>Hold </span><strong style={{ color: tokens.colors.textPrimary }}>{config.hotkey}</strong><span> and speak.</span></>,
    <>Release keys to transcribe and type.</>,
  ];

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto' }} key="status">
      <div style={{ ...tabPanelPaddedStyle, flex: 1 }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
          <StatusIcon status={currentStatus} large />
          <div style={{ fontSize: '20px', fontWeight: 700 }} key={`text-${currentStatus}`}>
            {currentStatus === 'Transcribing' ? `Transcribing (${config.transcription_mode})` : currentStatus}
          </div>
          <div style={{ width: '100%', maxWidth: '520px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <ModeSwitcher
              value={config.output_method}
              onToggle={onToggleOutputMethod}
              options={[
                { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter Mode: Simulates key presses' },
                { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard Mode: Fast copy-paste' },
              ]}
            />
            <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, opacity: 0.7, textAlign: 'center' }} key={`desc-${config.output_method}`}>
              {config.output_method === 'Typewriter'
                ? 'Types directly into your active cursor.'
                : 'Copies results to your clipboard.'}
            </div>
          </div>
        </div>

        <Card>
          <div style={{ fontSize: tokens.typography.sizeSm, lineHeight: 1.6, color: tokens.colors.textPrimary, padding: `${tokens.spacing.sm} ${tokens.spacing.md}` }}>
            <h3 style={{ margin: `0 0 ${tokens.spacing.sm} 0`, fontSize: tokens.typography.sizeLg, fontWeight: 700, textAlign: 'center', color: tokens.colors.textPrimary }}>
              How to Use Voquill
            </h3>
            <ol style={{ listStyle: 'none', margin: 0, padding: 0, textAlign: 'left' }}>
              {howToSteps.map((step, index) => (
                <li key={index} style={{ display: 'grid', gridTemplateColumns: '24px 1fr', alignItems: 'start', marginBottom: tokens.spacing.sm, color: tokens.colors.textSecondary }}>
                  <span style={{ color: tokens.colors.accentPrimary, fontWeight: 800, fontFamily: tokens.typography.fontMono, fontSize: tokens.typography.sizeMd }}>
                    {index + 1}.
                  </span>
                  <span>{step}</span>
                </li>
              ))}
            </ol>
          </div>
        </Card>

        <div style={{ marginTop: 'auto', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: tokens.spacing.xs, padding: `${tokens.spacing.xs} 0`, opacity: 0.6, transition: tokens.transitions.fast }}>
          <div style={{ display: 'flex', gap: tokens.spacing.sm, alignItems: 'center' }}>
            <button
              type="button"
              onClick={() => open('https://github.com/jackbrumley/voquill')}
              onMouseEnter={() => setHoveredFooterIcon('github')}
              onMouseLeave={() => setHoveredFooterIcon(null)}
              title="GitHub Repository"
              style={{
                background: hoveredFooterIcon === 'github' ? 'rgba(255, 255, 255, 0.05)' : 'none',
                border: 'none',
                padding: '10px',
                cursor: 'pointer',
                color: hoveredFooterIcon === 'github' ? tokens.colors.textPrimary : tokens.colors.textMuted,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: '50%',
                transform: hoveredFooterIcon === 'github' ? 'translateY(-2px)' : 'translateY(0)',
                transition: tokens.transitions.fast,
              }}
            >
              <IconBrandGithub size={20} />
            </button>
            <button
              type="button"
              onClick={() => open('https://voquill.org/donate')}
              onMouseEnter={() => setHoveredFooterIcon('heart')}
              onMouseLeave={() => setHoveredFooterIcon(null)}
              title="Support the project"
              style={{
                background: hoveredFooterIcon === 'heart' ? 'rgba(255, 255, 255, 0.05)' : 'none',
                border: 'none',
                padding: '10px',
                cursor: 'pointer',
                color: tokens.colors.textMuted,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                borderRadius: '50%',
                transform: hoveredFooterIcon === 'heart' ? 'translateY(-2px)' : 'translateY(0)',
                transition: tokens.transitions.fast,
              }}
            >
              <IconHeart
                size={20}
                color={hoveredFooterIcon === 'heart' ? '#ff4d5e' : '#ff6b6b'}
                fill={hoveredFooterIcon === 'heart' ? '#ff4d5e' : '#ff6b6b'}
                fillOpacity={hoveredFooterIcon === 'heart' ? 0.38 : 0.2}
              />
            </button>
          </div>
          <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, fontFamily: tokens.typography.fontMono }}>v{appVersion}</div>
          {hasUpdateAvailable && (
            <button
              type="button"
              onClick={onOpenUpdateModal}
              title="Open update details"
              style={{
                border: '1px solid rgba(255, 255, 255, 0.16)',
                background: tokens.colors.accentPrimary,
                cursor: 'pointer',
                padding: '4px 10px',
                borderRadius: '999px',
                color: '#ffffff',
                fontSize: tokens.typography.sizeXs,
                fontWeight: 800,
                letterSpacing: '0.01em',
                boxShadow: '0 6px 16px rgba(0, 0, 0, 0.3)',
                transition: tokens.transitions.fast,
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              <IconAlertCircle size={13} stroke={2.2} />
              <span>Update available</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
