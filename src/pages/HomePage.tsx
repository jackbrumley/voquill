import { IconAlertCircle, IconBrandGithub, IconHeart, IconUpload, IconCopy, IconUser } from '@tabler/icons-preact';
import { open } from '@tauri-apps/plugin-shell';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Card } from '../components/Card.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { GlassOrb } from '../components/GlassOrb.tsx';
import { JumpingDot } from '../components/JumpingDot.tsx';
import { AudioWave } from '../components/AudioWave.tsx';
import { BouncingDots } from '../components/BouncingDots.tsx';
import { tabPanelPaddedStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import { useEffect, useState } from 'preact/hooks';
import type { Config, DictationStatus, Segment } from '../types.ts';
import { getSpeakerColor } from '../speakerColors.ts';

interface HomePageProps {
  appVersion: string;
  dictationStatus: DictationStatus;
  config: Config;
  onToggleOutputMethod: (method: 'Typewriter' | 'Clipboard') => void;
  onToggleDiarization: (enabled: boolean) => void;
  hasUpdateAvailable: boolean;
  onOpenUpdateModal: () => void;
  onCopyToClipboard: (text: string) => void;
}

type ImportStatus = 'idle' | 'transcribing' | 'done' | 'error';

interface TranscribeResult {
  text: string;
  segments: Segment[];
  provider: string;
}

export function HomePage({
  appVersion,
  dictationStatus,
  config,
  onToggleOutputMethod,
  onToggleDiarization,
  hasUpdateAvailable,
  onOpenUpdateModal,
  onCopyToClipboard,
}: HomePageProps) {
  const [hoveredFooterIcon, setHoveredFooterIcon] = useState<'github' | 'heart' | null>(null);
  const importStatus = useSignal<ImportStatus>('idle');
  const importResult = useSignal<TranscribeResult | null>(null);
  const importError = useSignal<string>('');
  const isDragOver = useSignal(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    getCurrentWindow().onDragDropEvent((event) => {
      if (cancelled) return;
      if (event.payload.type === 'enter' || event.payload.type === 'over') {
        isDragOver.value = true;
      } else if (event.payload.type === 'leave') {
        isDragOver.value = false;
      } else if (event.payload.type === 'drop') {
        isDragOver.value = false;
        const path = event.payload.paths[0];
        if (path) {
          transcribeFile(path);
        }
      }
    }).then((fn) => {
      if (cancelled) { fn(); return; }
      unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handleFilePick = async () => {
    try {
      const selected = await openFileDialog({
        multiple: false,
        filters: [{
          name: 'Audio',
          extensions: ['wav', 'mp3', 'm4a', 'ogg', 'flac'],
        }],
      });
      if (selected) {
        await transcribeFile(selected);
      }
    } catch (e) {
      importError.value = `Failed to pick file: ${e}`;
      importStatus.value = 'error';
    }
  };

  const transcribeFile = async (filePath: string) => {
    importStatus.value = 'transcribing';
    importResult.value = null;
    importError.value = '';
    try {
      const result = await invoke<TranscribeResult>('transcribe_audio_file', { path: filePath });
      importResult.value = result;
      importStatus.value = 'done';
    } catch (e) {
      importError.value = `Transcription failed: ${e}`;
      importStatus.value = 'error';
    }
  };

  const copyLabeledText = () => {
    const result = importResult.value;
    if (!result) return;
    onCopyToClipboard(result.text);
  };

  const copyPlainText = () => {
    const result = importResult.value;
    if (!result) return;
    const plain = result.segments && result.segments.length > 0
      ? result.segments.map(s => s.text).join(' ')
      : result.text;
    onCopyToClipboard(plain);
  };

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto' }} key="home">
      <div style={{ ...tabPanelPaddedStyle, flex: 1 }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '10px' }}>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', paddingTop: '16px' }}>
              <GlassOrb size={80}>
              <div style={{ position: 'relative', width: '100%', height: '100%' }}>
                <div style={{ position: 'absolute', inset: 0, opacity: dictationStatus === 'Ready' ? 1 : 0, transition: 'opacity 0.2s ease', pointerEvents: dictationStatus === 'Ready' ? 'auto' : 'none' }}>
                  <JumpingDot dotSize={24} jumpHeight={30} />
                </div>
                <div style={{ position: 'absolute', inset: 0, opacity: dictationStatus === 'Recording' ? 1 : 0, transition: 'opacity 0.2s ease', pointerEvents: dictationStatus === 'Recording' ? 'auto' : 'none' }}>
                  <AudioWave containerHeight={80} barWidth={5} gap={3} />
                </div>
                <div style={{ position: 'absolute', inset: 0, opacity: (dictationStatus === 'Transcribing' || dictationStatus === 'Processing' || dictationStatus === 'Typing') ? 1 : 0, transition: 'opacity 0.2s ease', pointerEvents: (dictationStatus === 'Transcribing' || dictationStatus === 'Processing' || dictationStatus === 'Typing') ? 'auto' : 'none' }}>
                  <BouncingDots dotSize={16} jumpHeight={18} />
                </div>
              </div>
            </GlassOrb>
              {dictationStatus && (
                <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', marginTop: '6px', marginBottom: '8px' }}>
                  <div style={{
                    fontSize: '28px',
                    fontWeight: 700,
                    color: tokens.colors.textPrimary,
                    letterSpacing: '-0.02em',
                    lineHeight: 1.2,
                    textAlign: 'center',
                  }}>
                    {dictationStatus === 'Error' ? 'Mic not found' : dictationStatus}
                  </div>
                  {dictationStatus === 'Ready' && config.hotkey ? (
                    <div style={{
                      fontSize: tokens.typography.sizeSm,
                      color: '#e2e8f0',
                      textAlign: 'center',
                      marginTop: '4px',
                      letterSpacing: '0.01em',
                    }}>
                      {config.hotkey_mode === 'Toggle' ? 'Press ' : 'Hold '}
                      <span style={{ color: '#818cf8', fontWeight: 600 }}>
                        {config.hotkey}
                      </span>
                      {' to start dictation'}
                    </div>
                  ) : (
                    <div style={{
                      fontSize: tokens.typography.sizeSm,
                      color: '#94a3b8',
                      textAlign: 'center',
                      marginTop: '4px',
                      minHeight: '18px',
                    }}>
                      {dictationStatus === 'Recording' ? 'Listening...' : (dictationStatus === 'Transcribing' || dictationStatus === 'Processing' || dictationStatus === 'Typing') ? 'Transcribing speech...' : ''}
                    </div>
                  )}
                </div>
              )}
            </div>
          <div style={{ width: '100%', maxWidth: '520px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <div style={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
              <ModeSwitcher
                value={config.output_method}
                onToggle={onToggleOutputMethod}
                options={[
                  { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter Mode: Simulates key presses' },
                  { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard Mode: Fast copy-paste' },
                ]}
              />
            </div>
            <div style={{ fontSize: tokens.typography.sizeXs, color: '#cbd5e1', opacity: 0.9, textAlign: 'center' }} key={`desc-${config.output_method}`}>
              {config.output_method === 'Typewriter'
                ? 'Types directly into your active cursor.'
                : 'Copies results to your clipboard.'}
            </div>
            </div>
        </div>

        <div style={{
          fontSize: '11px',
          fontWeight: 600,
          color: '#cbd5e1',
          letterSpacing: '0.06em',
          textTransform: 'uppercase',
          marginBottom: '-6px',
          paddingLeft: '2px',
        }}>
          Transcribe an Audio File
        </div>

        <Card
          onClick={handleFilePick}
          style={{
            padding: '14px',
            boxShadow: isDragOver.value ? tokens.shadows.accent : undefined,
          }}
        >
          {importStatus.value === 'idle' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
              <div style={{ display: 'flex', flexDirection: 'row', alignItems: 'center', width: '100%', gap: tokens.spacing.md }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: tokens.typography.sizeSm, color: '#ffffff', fontWeight: 500, lineHeight: 1.4 }}>
                    Drop an audio file here or click to browse
                  </div>
                  <div style={{ fontSize: tokens.typography.sizeXs, color: '#94a3b8', marginTop: '2px' }}>
                    WAV, MP3, M4A, OGG, FLAC, Opus
                  </div>
                </div>
                <div style={{
                  width: '36px',
                  height: '36px',
                  borderRadius: '10px',
                  background: 'rgba(88, 101, 242, 0.12)',
                  border: '1px solid rgba(88, 101, 242, 0.25)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: '#818cf8',
                  flexShrink: 0,
                }}>
                  <IconUpload size={20} />
                </div>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px', borderTop: '1px solid rgba(255,255,255,0.06)', paddingTop: '8px' }} onClick={(e) => e.stopPropagation()}>
                <label style={{
                  fontSize: tokens.typography.sizeXs,
                  color: '#e2e8f0',
                  fontWeight: 500,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  userSelect: 'none',
                }}>
                  <input
                    type="checkbox"
                    checked={config.diarization_enabled_files}
                    onChange={(e) => { e.stopPropagation(); onToggleDiarization((e.target as HTMLInputElement).checked); }}
                    style={{ accentColor: tokens.colors.accentPrimary, cursor: 'pointer' }}
                  />
                  <IconUser size={13} style={{ color: '#94a3b8' }} />
                  Differentiate voices
                </label>
                <span style={{ fontSize: tokens.typography.sizeXs, color: '#94a3b8' }}>
                  Labels each speaker
                </span>
              </div>
            </div>
          )}
          {importStatus.value === 'transcribing' && (
            <div style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.accentPrimary }}>
              Transcribing...
            </div>
          )}
          {importStatus.value === 'done' && importResult.value && (
            <>
              <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.success || '#43b581', marginBottom: '4px' }}>
                Transcription complete {importResult.value.provider !== 'none' && `(${importResult.value.provider})`}
              </div>
              <div style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textPrimary, lineHeight: 1.45, maxHeight: '100px', overflow: 'auto', width: '100%', textAlign: 'left' }}>
                {importResult.value.segments && importResult.value.segments.length > 0
                  ? importResult.value.segments.map((seg, i) => (
                    <div key={i} style={{ marginBottom: '2px' }}>
                      {seg.speaker && (
                        <span style={{ color: getSpeakerColor(seg.speaker), fontWeight: 800, fontSize: tokens.typography.sizeXs }}>
                          [{seg.speaker}]
                        </span>
                      )}
                      {' '}{seg.text}
                    </div>
                  ))
                  : importResult.value.text
                }
              </div>
              <div style={{ display: 'flex', gap: tokens.spacing.sm, marginTop: tokens.spacing.xs }}>
                <button
                  onClick={(e) => { e.stopPropagation(); copyLabeledText(); }}
                  title="Copy with speaker labels"
                  style={{
                    background: 'transparent',
                    border: 'none',
                    padding: '6px 10px',
                    borderRadius: '6px',
                    color: '#a9acb5',
                    cursor: 'pointer',
                    fontSize: tokens.typography.sizeXs,
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    transition: 'all 0.15s ease',
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.1)';
                    (e.currentTarget as HTMLElement).style.color = '#ffffff';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = 'transparent';
                    (e.currentTarget as HTMLElement).style.color = '#a9acb5';
                  }}
                >
                  <IconCopy size={16} />
                  Copy labeled
                </button>
                <button
                  onClick={(e) => { e.stopPropagation(); copyPlainText(); }}
                  title="Copy without labels"
                  style={{
                    background: 'transparent',
                    border: 'none',
                    padding: '6px 10px',
                    borderRadius: '6px',
                    color: '#a9acb5',
                    cursor: 'pointer',
                    fontSize: tokens.typography.sizeXs,
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    transition: 'all 0.15s ease',
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.1)';
                    (e.currentTarget as HTMLElement).style.color = '#ffffff';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = 'transparent';
                    (e.currentTarget as HTMLElement).style.color = '#a9acb5';
                  }}
                >
                  <IconCopy size={16} />
                  Copy plain
                </button>
                <button
                  onClick={(e) => { e.stopPropagation(); importStatus.value = 'idle'; }}
                  title="Transcribe another file"
                  style={{
                    background: 'transparent',
                    border: 'none',
                    padding: '6px 10px',
                    borderRadius: '6px',
                    color: '#a9acb5',
                    cursor: 'pointer',
                    fontSize: tokens.typography.sizeXs,
                    transition: 'all 0.15s ease',
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.1)';
                    (e.currentTarget as HTMLElement).style.color = '#ffffff';
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = 'transparent';
                    (e.currentTarget as HTMLElement).style.color = '#a9acb5';
                  }}
                >
                  <IconUpload size={16} />
                  New Import
                </button>
              </div>
            </>
          )}
          {importStatus.value === 'error' && (
            <>
              <div style={{ fontSize: tokens.typography.sizeSm, color: '#f04747', marginBottom: '4px' }}>
                {importError.value}
              </div>
              <button
                onClick={(e) => { e.stopPropagation(); importStatus.value = 'idle'; }}
                style={{
                  background: 'rgba(255,255,255,0.08)',
                  border: 'none',
                  padding: '4px 10px',
                  borderRadius: '6px',
                  color: tokens.colors.textSecondary,
                  cursor: 'pointer',
                  fontSize: tokens.typography.sizeXs,
                }}
              >
                Try Again
              </button>
            </>
          )}
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