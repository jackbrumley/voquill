import { IconClock, IconX } from '@tabler/icons-preact';
import type { MacroStep } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';

interface MacroStepChipProps {
  step: MacroStep;
  onRemove?: () => void;
  isEditingDelay?: boolean;
  editingDelayValue?: string;
  onStartEditDelay?: () => void;
  onDelayInputChange?: (val: string) => void;
  onSaveDelay?: () => void;
}

export function MacroStepChip({
  step,
  onRemove,
  isEditingDelay = false,
  editingDelayValue = '',
  onStartEditDelay,
  onDelayInputChange,
  onSaveDelay,
}: MacroStepChipProps) {
  if (step.type === 'KeyPress') {
    return (
      <span
        style={{
          padding: '3px 8px',
          borderRadius: '4px',
          background: 'rgba(88, 101, 242, 0.25)',
          border: '1px solid rgba(88, 101, 242, 0.45)',
          fontSize: '12px',
          fontWeight: 600,
          color: '#9ba5ff',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <span>⌨️ {step.key}</span>
        <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
          ({step.hold_ms || 50}ms)
        </span>
        {onRemove && (
          <button
            type="button"
            onClick={onRemove}
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              cursor: 'pointer',
              padding: '0 2px',
            }}
          >
            <IconX size={10} />
          </button>
        )}
      </span>
    );
  }

  if (step.type === 'KeyDown') {
    return (
      <span
        style={{
          padding: '3px 8px',
          borderRadius: '4px',
          background: 'rgba(168, 85, 247, 0.25)',
          border: '1px solid rgba(168, 85, 247, 0.45)',
          fontSize: '12px',
          fontWeight: 600,
          color: '#c084fc',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <span>⬇️ Hold: {step.key}</span>
        {onRemove && (
          <button
            type="button"
            onClick={onRemove}
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              cursor: 'pointer',
              padding: '0 2px',
            }}
          >
            <IconX size={10} />
          </button>
        )}
      </span>
    );
  }

  if (step.type === 'KeyUp') {
    return (
      <span
        style={{
          padding: '3px 8px',
          borderRadius: '4px',
          background: 'rgba(147, 51, 234, 0.25)',
          border: '1px solid rgba(147, 51, 234, 0.45)',
          fontSize: '12px',
          fontWeight: 600,
          color: '#d8b4fe',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <span>⬆️ Rel: {step.key}</span>
        {onRemove && (
          <button
            type="button"
            onClick={onRemove}
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              cursor: 'pointer',
              padding: '0 2px',
            }}
          >
            <IconX size={10} />
          </button>
        )}
      </span>
    );
  }

  if (step.type === 'Delay') {
    return (
      <span
        style={{
          padding: '2px 6px',
          borderRadius: '4px',
          background: 'rgba(234, 179, 8, 0.18)',
          border: '1px solid rgba(234, 179, 8, 0.35)',
          fontSize: '11px',
          fontWeight: 500,
          color: '#fde047',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <IconClock size={11} />
        {isEditingDelay && onDelayInputChange && onSaveDelay ? (
          <input
            type="number"
            value={editingDelayValue}
            onInput={(e) => onDelayInputChange((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') onSaveDelay();
            }}
            onBlur={onSaveDelay}
            autoFocus
            style={{
              width: '45px',
              background: '#000',
              border: '1px solid #fde047',
              color: '#fff',
              fontSize: '11px',
              padding: '1px 2px',
              borderRadius: '2px',
            }}
          />
        ) : (
          <span
            onClick={onStartEditDelay}
            title="Click to edit delay milliseconds"
            style={{ cursor: onStartEditDelay ? 'pointer' : 'default', textDecoration: onStartEditDelay ? 'underline dotted' : 'none' }}
          >
            {step.duration_ms}ms
          </span>
        )}
        {onRemove && (
          <button
            type="button"
            onClick={onRemove}
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              cursor: 'pointer',
              padding: '0 2px',
            }}
          >
            <IconX size={10} />
          </button>
        )}
      </span>
    );
  }

  if (step.type === 'TypeText') {
    return (
      <span
        style={{
          padding: '3px 8px',
          borderRadius: '4px',
          background: 'rgba(59, 130, 246, 0.25)',
          border: '1px solid rgba(59, 130, 246, 0.45)',
          fontSize: '12px',
          fontWeight: 600,
          color: '#93c5fd',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
        }}
      >
        <span>📝 "{step.text}"</span>
        {onRemove && (
          <button
            type="button"
            onClick={onRemove}
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              cursor: 'pointer',
              padding: '0 2px',
            }}
          >
            <IconX size={10} />
          </button>
        )}
      </span>
    );
  }

  return null;
}
