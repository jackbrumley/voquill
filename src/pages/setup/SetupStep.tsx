import { ComponentChildren } from 'preact';
import { IconCheck } from '@tabler/icons-preact';
import { tokens } from '../../design-tokens.ts';

export type SetupStepState = 'done' | 'current' | 'upcoming';

interface SetupStepProps {
  number: number;
  title: string;
  /** One line shown under the title while the step is folded, e.g. the chosen microphone. */
  summary?: string;
  state: SetupStepState;
  isOpen: boolean;
  /** Finished steps can be reopened to change them. */
  onToggle?: () => void;
  children: ComponentChildren;
}

/// One numbered step of initial setup. Only the open step shows its
/// content; finished steps fold to a tick and a one-line summary.
export function SetupStep({ number, title, summary, state, isOpen, onToggle, children }: SetupStepProps) {
  const isDone = state === 'done';
  const isUpcoming = state === 'upcoming';
  const canToggle = isDone && onToggle !== undefined;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm }}>
      <button
        type="button"
        disabled={!canToggle}
        aria-expanded={isOpen}
        onClick={onToggle}
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          gap: '10px',
          border: 'none',
          background: 'transparent',
          padding: 0,
          textAlign: 'left',
          cursor: canToggle ? 'pointer' : 'default',
          color: 'inherit',
        }}
      >
        <span style={{
          width: '22px',
          height: '22px',
          borderRadius: '50%',
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: tokens.typography.sizeXs,
          fontWeight: 700,
          background: isDone ? tokens.colors.success : state === 'current' ? tokens.colors.accentPrimary : 'transparent',
          border: isUpcoming ? `1px solid ${tokens.colors.textMuted}` : 'none',
          color: isUpcoming ? tokens.colors.textMuted : tokens.colors.textPrimary,
        }}>
          {isDone ? <IconCheck size={14} stroke={3} /> : number}
        </span>
        <span style={{ display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <span style={{
            fontSize: tokens.typography.sizeMd,
            fontWeight: 600,
            lineHeight: '22px',
            color: isUpcoming ? tokens.colors.textMuted : tokens.colors.textPrimary,
          }}>
            {title}
          </span>
          {summary && !isOpen && (
            <span style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>{summary}</span>
          )}
        </span>
      </button>
      {isOpen && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm, paddingLeft: '32px' }}>
          {children}
        </div>
      )}
    </div>
  );
}
