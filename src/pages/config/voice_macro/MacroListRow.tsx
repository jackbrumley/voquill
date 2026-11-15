import { ComponentChildren } from 'preact';
import {
  IconCopy,
  IconMicrophone,
  IconPencil,
  IconPlayerPlay,
  IconShare,
  IconUpload,
  IconVolume,
  IconVolumeOff,
} from '@tabler/icons-preact';
import { tokens } from '../../../design-tokens.ts';
import type { VoiceMacroCommand } from '../../../types.ts';
import { resolveMacroSteps } from './keyUtils.ts';

interface MacroListRowProps {
  macro: VoiceMacroCommand;
  triggerWord: string;
  isTesting: boolean;
  onTest: () => void;
  onDuplicate: () => void;
  onShare: () => void;
  onEdit: () => void;
}

const actionButtonStyle = {
  background: 'none',
  border: 'none',
  color: tokens.colors.textSecondary,
  cursor: 'pointer',
  padding: '3px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  borderRadius: '3px',
};

interface BadgeColors {
  color: string;
  background: string;
  border: string;
}

const BADGE_COLORS = {
  prefix: { color: '#93c5fd', background: 'rgba(59, 130, 246, 0.18)', border: 'rgba(59, 130, 246, 0.35)' },
  neutral: { color: '#cbd5e1', background: 'rgba(255, 255, 255, 0.06)', border: 'rgba(255, 255, 255, 0.12)' },
  spoken: { color: '#f472b6', background: 'rgba(236, 72, 153, 0.15)', border: 'rgba(236, 72, 153, 0.3)' },
  audioFile: { color: '#34d399', background: 'rgba(16, 185, 129, 0.15)', border: 'rgba(16, 185, 129, 0.3)' },
  voiceClip: { color: '#f87171', background: 'rgba(239, 68, 68, 0.15)', border: 'rgba(239, 68, 68, 0.3)' },
  muted: { color: tokens.colors.textMuted, background: 'rgba(255, 255, 255, 0.05)', border: 'rgba(255, 255, 255, 0.1)' },
} satisfies Record<string, BadgeColors>;

interface BadgeProps {
  colors: BadgeColors;
  title?: string;
  maxWidth?: string;
  children: ComponentChildren;
}

function Badge({ colors, title, maxWidth, children }: BadgeProps) {
  return (
    <span
      title={title}
      style={{
        fontSize: '9.5px',
        color: colors.color,
        background: colors.background,
        border: `1px solid ${colors.border}`,
        padding: '1px 5px',
        borderRadius: '4px',
        whiteSpace: 'nowrap',
        maxWidth,
        overflow: maxWidth ? 'hidden' : undefined,
        textOverflow: maxWidth ? 'ellipsis' : undefined,
        display: 'inline-flex',
        alignItems: 'center',
        gap: '3px',
      }}
    >
      {children}
    </span>
  );
}

/// The badge describing what a macro plays when it fires.
function SoundBadge({ macro }: { macro: VoiceMacroCommand }) {
  switch (macro.sound_mode) {
    case 'tts':
      return macro.sound_tts_text ? (
        <Badge colors={BADGE_COLORS.spoken} maxWidth="140px" title={`Voice feedback: "${macro.sound_tts_text}"`}>
          <IconVolume size={11} />
          <span>"{macro.sound_tts_text}"</span>
        </Badge>
      ) : null;
    case 'custom_file':
      return (
        <Badge colors={BADGE_COLORS.audioFile}>
          <IconUpload size={11} />
          <span>Audio</span>
        </Badge>
      );
    case 'mic_recording':
      return (
        <Badge colors={BADGE_COLORS.voiceClip}>
          <IconMicrophone size={11} />
          <span>Voice Clip</span>
        </Badge>
      );
    case 'none':
      return (
        <Badge colors={BADGE_COLORS.muted}>
          <IconVolumeOff size={11} />
          <span>Mute</span>
        </Badge>
      );
    default:
      return null;
  }
}

/// One macro in the library list: its phrase, a summary, and its actions.
export function MacroListRow({ macro, triggerWord, isTesting, onTest, onDuplicate, onShare, onEdit }: MacroListRowProps) {
  const stepCount = resolveMacroSteps(macro).length;
  const aliases = macro.phrases ?? [];

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '8px 10px',
        borderRadius: '6px',
        background: 'rgba(255, 255, 255, 0.04)',
        border: '1px solid rgba(255, 255, 255, 0.08)',
        gap: '6px',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap', minWidth: 0, flex: 1 }}>
        <span
          style={{
            fontSize: '13px',
            color: tokens.colors.textPrimary,
            fontWeight: 600,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          "{macro.phrase}"
        </span>
        {triggerWord && <Badge colors={BADGE_COLORS.prefix}>Prefix: {triggerWord}</Badge>}
        {aliases.length > 0 && (
          <Badge colors={BADGE_COLORS.neutral} title={`Aliases: ${aliases.map((p) => `"${p}"`).join(', ')}`}>
            +{aliases.length} {aliases.length === 1 ? 'alias' : 'aliases'}
          </Badge>
        )}
        <span style={{ fontSize: '11px', color: tokens.colors.textMuted, whiteSpace: 'nowrap' }}>
          • {stepCount} {stepCount === 1 ? 'step' : 'steps'}
        </span>
        <SoundBadge macro={macro} />
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '3px', flexShrink: 0 }}>
        <button onClick={onTest} disabled={isTesting} style={actionButtonStyle} title="Test macro sequence">
          <IconPlayerPlay size={14} />
        </button>
        <button onClick={onDuplicate} style={actionButtonStyle} title={`Duplicate "${macro.phrase}"`}>
          <IconCopy size={14} />
        </button>
        <button onClick={onShare} style={actionButtonStyle} title={`Copy "${macro.phrase}" JSON to clipboard`}>
          <IconShare size={14} />
        </button>
        <button onClick={onEdit} style={actionButtonStyle} title={`Edit "${macro.phrase}" sequence`}>
          <IconPencil size={14} />
        </button>
      </div>
    </div>
  );
}
