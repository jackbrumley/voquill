import { IconX, IconClock, IconKeyboard, IconArrowDown, IconArrowUp, IconWriting } from '@tabler/icons-preact';
import type { MacroStep } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';

interface MacroStepRowProps {
  index: number;
  step: MacroStep;
  onRemove: () => void;
  isEditingDelay?: boolean;
  editingDelayValue?: string;
  onStartEditDelay?: () => void;
  onDelayInputChange?: (val: string) => void;
  onSaveDelay?: () => void;
}

export function MacroStepRow({
  index,
  step,
  onRemove,
  isEditingDelay = false,
  editingDelayValue = '',
  onStartEditDelay,
  onDelayInputChange,
  onSaveDelay,
}: MacroStepRowProps) {
  const renderBadgeAndContent = () => {
    switch (step.type) {
      case 'KeyPress':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 600,
                background: 'rgba(88, 101, 242, 0.18)',
                color: '#818cf8',
                border: '1px solid rgba(88, 101, 242, 0.35)',
                minWidth: '78px',
                justifyContent: 'center',
              }}
            >
              <IconKeyboard size={12} />
              <span>Tap Key</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '13px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '2px 8px',
                borderRadius: '4px',
                border: '1px solid rgba(255, 255, 255, 0.12)',
              }}
            >
              {step.key}
            </span>

            {step.hold_ms && step.hold_ms !== 50 && (
              <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
                (held {step.hold_ms} ms)
              </span>
            )}
          </>
        );

      case 'KeyDown':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 600,
                background: 'rgba(245, 158, 11, 0.18)',
                color: '#fbbf24',
                border: '1px solid rgba(245, 158, 11, 0.35)',
                minWidth: '78px',
                justifyContent: 'center',
              }}
            >
              <IconArrowDown size={12} />
              <span>Hold Down</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '13px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '2px 8px',
                borderRadius: '4px',
                border: '1px solid rgba(255, 255, 255, 0.12)',
              }}
            >
              {step.key}
            </span>
          </>
        );

      case 'KeyUp':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 600,
                background: 'rgba(168, 85, 247, 0.18)',
                color: '#c084fc',
                border: '1px solid rgba(168, 85, 247, 0.35)',
                minWidth: '78px',
                justifyContent: 'center',
              }}
            >
              <IconArrowUp size={12} />
              <span>Release</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '13px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '2px 8px',
                borderRadius: '4px',
                border: '1px solid rgba(255, 255, 255, 0.12)',
              }}
            >
              {step.key}
            </span>
          </>
        );

      case 'Delay':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 600,
                background: 'rgba(14, 165, 233, 0.18)',
                color: '#38bdf8',
                border: '1px solid rgba(14, 165, 233, 0.35)',
                minWidth: '78px',
                justifyContent: 'center',
              }}
            >
              <IconClock size={12} />
              <span>Delay</span>
            </span>

            {isEditingDelay ? (
              <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                <input
                  type="number"
                  value={editingDelayValue}
                  onInput={(e) => onDelayInputChange?.((e.target as HTMLInputElement).value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') onSaveDelay?.();
                    if (e.key === 'Escape') onSaveDelay?.();
                  }}
                  onBlur={onSaveDelay}
                  autoFocus
                  style={{
                    ...inputBaseStyle,
                    width: '65px',
                    padding: '2px 6px',
                    fontSize: '12px',
                    fontFamily: 'monospace',
                    textAlign: 'center',
                  }}
                />
                <span style={{ fontSize: '12px', color: tokens.colors.textMuted }}>ms</span>
              </div>
            ) : (
              <button
                type="button"
                onClick={onStartEditDelay}
                title="Click to edit delay duration"
                style={{
                  background: 'rgba(255, 255, 255, 0.04)',
                  border: '1px dashed rgba(255, 255, 255, 0.18)',
                  borderRadius: '4px',
                  padding: '2px 8px',
                  fontSize: '12px',
                  fontFamily: 'monospace',
                  color: tokens.colors.textSecondary,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                }}
              >
                <span>{step.duration_ms} ms</span>
                <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>(edit)</span>
              </button>
            )}
          </>
        );

      case 'TypeText':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 8px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 600,
                background: 'rgba(16, 185, 129, 0.18)',
                color: '#34d399',
                border: '1px solid rgba(16, 185, 129, 0.35)',
                minWidth: '78px',
                justifyContent: 'center',
              }}
            >
              <IconWriting size={12} />
              <span>Type Text</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '12px',
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '2px 8px',
                borderRadius: '4px',
              }}
            >
              "{step.text}"
            </span>
          </>
        );
    }
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '6px 10px',
        borderRadius: '6px',
        background: 'rgba(255, 255, 255, 0.03)',
        border: '1px solid rgba(255, 255, 255, 0.06)',
        gap: '8px',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flex: 1, minWidth: 0 }}>
        <span
          style={{
            fontSize: '11px',
            fontFamily: 'monospace',
            color: tokens.colors.textMuted,
            minWidth: '20px',
          }}
        >
          {index + 1}.
        </span>

        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' }}>
          {renderBadgeAndContent()}
        </div>
      </div>

      <button
        type="button"
        onClick={onRemove}
        title="Remove step"
        style={{
          background: 'none',
          border: 'none',
          color: tokens.colors.textMuted,
          cursor: 'pointer',
          padding: '4px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '4px',
        }}
      >
        <IconX size={14} />
      </button>
    </div>
  );
}
