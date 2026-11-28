import {
  IconBolt,
  IconCopy,
  IconMicrophone,
  IconPencil,
  IconPlayerPlay,
  IconShare,
  IconUpload,
  IconVolume,
  IconVolumeOff,
} from '@tabler/icons-preact';
import { Switch } from '../../../components/Switch.tsx';
import { tokens } from '../../../design-tokens.ts';
import type { VoiceMacroCommand } from '../../../types.ts';
import { resolveMacroSteps } from './keyUtils.ts';

interface MacroListRowProps {
  macro: VoiceMacroCommand;
  triggerWord: string;
  isTesting: boolean;
  onEnabledChange: (enabled: boolean) => void;
  onRun: () => void;
  onPlaySound: () => void;
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

interface SoundStyle {
  icon: typeof IconVolume;
  color: string;
  label: string;
}

/// What a macro plays when it fires, as one small icon. Default chime and
/// silence-with-no-setting show nothing.
function soundStyle(macro: VoiceMacroCommand): SoundStyle | null {
  switch (macro.sound_mode) {
    case 'tts':
      return macro.sound_tts_text
        ? { icon: IconVolume, color: '#f472b6', label: `Says "${macro.sound_tts_text}"` }
        : null;
    case 'custom_file':
      return { icon: IconUpload, color: '#34d399', label: 'Plays an audio file' };
    case 'mic_recording':
      return { icon: IconMicrophone, color: '#f87171', label: 'Plays a voice clip' };
    case 'none':
      return { icon: IconVolumeOff, color: tokens.colors.textMuted, label: 'No sound' };
    default:
      return null;
  }
}

/// Everything the row leaves out, for its tooltip.
function rowDetails(macro: VoiceMacroCommand, triggerWord: string, sound: SoundStyle | null): string {
  const lines = [`Say: "${triggerWord ? `${triggerWord} ` : ''}${macro.phrase}"`];
  if (!macro.enabled) {
    lines.push('Switched off: Voquill hears this phrase but does nothing.');
  }
  const aliases = macro.phrases ?? [];
  if (aliases.length > 0) {
    lines.push(`Also: ${aliases.map((alias) => `"${alias}"`).join(', ')}`);
  }
  if (sound) {
    lines.push(sound.label);
  }
  return lines.join('\n');
}

/// One macro in the library list: its phrase, a summary, and its actions.
export function MacroListRow({ macro, triggerWord, isTesting, onEnabledChange, onRun, onPlaySound, onDuplicate, onShare, onEdit }: MacroListRowProps) {
  const stepCount = resolveMacroSteps(macro).length;
  const sound = soundStyle(macro);
  const SoundIcon = sound?.icon;

  return (
    <div
      title={rowDetails(macro, triggerWord, sound)}
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
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', minWidth: 0, flex: 1 }}>
        <Switch
          name={`Voice macro "${macro.phrase}"`}
          checked={macro.enabled}
          onChange={onEnabledChange}
        />
        <span
          style={{
            fontSize: '13px',
            color: macro.enabled ? tokens.colors.textPrimary : tokens.colors.textMuted,
            fontWeight: 600,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          "{macro.phrase}"
        </span>
        {!macro.enabled && (
          <span style={{ fontSize: '11px', color: tokens.colors.textMuted, whiteSpace: 'nowrap' }}>Off</span>
        )}
        <span style={{ fontSize: '11px', color: tokens.colors.textMuted, whiteSpace: 'nowrap' }}>
          {stepCount} {stepCount === 1 ? 'step' : 'steps'}
        </span>
        {SoundIcon && sound && <SoundIcon size={12} color={sound.color} style={{ flexShrink: 0 }} />}
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '3px', flexShrink: 0 }}>
        {macro.sound_mode !== 'none' && (
          <button onClick={onPlaySound} style={actionButtonStyle} title="Play the sound this macro makes">
            <IconPlayerPlay size={14} />
          </button>
        )}
        <button onClick={onRun} disabled={isTesting} style={actionButtonStyle} title="Run this macro now">
          <IconBolt size={14} />
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
