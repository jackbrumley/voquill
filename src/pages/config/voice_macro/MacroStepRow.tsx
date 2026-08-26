import {
  IconX,
  IconClock,
  IconKeyboard,
  IconArrowDown,
  IconArrowUp,
  IconWriting,
  IconTerminal2,
  IconGripVertical,
  IconEdit,
} from '@tabler/icons-preact';
import type { MacroStep } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { DurationInput } from './MacroStepInputs.tsx';

interface MacroStepRowProps {
  index: number;
  step: MacroStep;
  onRemove: () => void;
  isEditingDuration?: boolean;
  editingDurationValue?: string;
  onStartEditDuration?: () => void;
  onDurationInputChange?: (val: string) => void;
  onSaveDuration?: () => void;
  onCancelDuration?: () => void;
  onStartEditStep?: () => void;
  canDrag?: boolean;
  isDragging?: boolean;
  onGripPointerDown?: (e: PointerEvent, index: number) => void;
  onGripPointerMove?: (e: PointerEvent) => void;
  onGripPointerUp?: (e: PointerEvent) => void;
  onGripPointerCancel?: (e: PointerEvent) => void;
}

export function MacroStepRow({
  index,
  step,
  onRemove,
  isEditingDuration = false,
  editingDurationValue = '',
  onStartEditDuration,
  onDurationInputChange,
  onSaveDuration,
  onCancelDuration,
  onStartEditStep,
  canDrag = true,
  isDragging = false,
  onGripPointerDown,
  onGripPointerMove,
  onGripPointerUp,
  onGripPointerCancel,
}: MacroStepRowProps) {
  const renderBadgeAndContent = () => {
    switch (step.type) {
      case 'KeyPress': {
        const holdMs = step.hold_ms || 50;
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '3px',
                padding: '1px 6px',
                borderRadius: '4px',
                fontSize: '10.5px',
                fontWeight: 600,
                background: 'rgba(88, 101, 242, 0.18)',
                color: '#818cf8',
                border: '1px solid rgba(88, 101, 242, 0.35)',
                minWidth: '54px',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              <IconKeyboard size={11} />
              <span>Tap</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '12px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '1px 6px',
                borderRadius: '4px',
                border: '1px solid rgba(255, 255, 255, 0.12)',
              }}
            >
              {step.key}
            </span>

            {isEditingDuration ? (
              <DurationInput
                value={editingDurationValue}
                onChange={(val) => onDurationInputChange?.(val)}
                onSave={() => onSaveDuration?.()}
                onCancel={onCancelDuration}
              />
            ) : (
              <button
                type="button"
                onClick={onStartEditDuration}
                title="Click to edit key hold duration"
                style={{
                  background: 'rgba(255, 255, 255, 0.04)',
                  border: '1px dashed rgba(255, 255, 255, 0.18)',
                  borderRadius: '4px',
                  padding: '1px 5px',
                  fontSize: '10.5px',
                  fontFamily: 'monospace',
                  color: tokens.colors.textSecondary,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '2px',
                }}
              >
                <span>{holdMs}ms</span>
                <span style={{ fontSize: '9px', color: tokens.colors.textMuted }}>(edit)</span>
              </button>
            )}
          </>
        );
      }

      case 'KeyDown':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '3px',
                padding: '1px 6px',
                borderRadius: '4px',
                fontSize: '10.5px',
                fontWeight: 600,
                background: 'rgba(245, 158, 11, 0.18)',
                color: '#fbbf24',
                border: '1px solid rgba(245, 158, 11, 0.35)',
                minWidth: '54px',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              <IconArrowDown size={11} />
              <span>Hold</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '12px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '1px 6px',
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
                gap: '3px',
                padding: '1px 6px',
                borderRadius: '4px',
                fontSize: '10.5px',
                fontWeight: 600,
                background: 'rgba(168, 85, 247, 0.18)',
                color: '#c084fc',
                border: '1px solid rgba(168, 85, 247, 0.35)',
                minWidth: '54px',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              <IconArrowUp size={11} />
              <span>Rel</span>
            </span>

            <span
              style={{
                fontFamily: 'monospace',
                fontSize: '12px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
                background: 'rgba(255, 255, 255, 0.08)',
                padding: '1px 6px',
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
                gap: '3px',
                padding: '1px 6px',
                borderRadius: '4px',
                fontSize: '10.5px',
                fontWeight: 600,
                background: 'rgba(14, 165, 233, 0.18)',
                color: '#38bdf8',
                border: '1px solid rgba(14, 165, 233, 0.35)',
                minWidth: '54px',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              <IconClock size={11} />
              <span>Delay</span>
            </span>

            {isEditingDuration ? (
              <DurationInput
                value={editingDurationValue}
                onChange={(val) => onDurationInputChange?.(val)}
                onSave={() => onSaveDuration?.()}
                onCancel={onCancelDuration}
              />
            ) : (
              <button
                type="button"
                onClick={onStartEditDuration}
                title="Click to edit delay duration"
                style={{
                  background: 'rgba(255, 255, 255, 0.04)',
                  border: '1px dashed rgba(255, 255, 255, 0.18)',
                  borderRadius: '4px',
                  padding: '1px 6px',
                  fontSize: '11px',
                  fontFamily: 'monospace',
                  color: tokens.colors.textSecondary,
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '3px',
                }}
              >
                <span>{step.duration_ms}ms</span>
                <span style={{ fontSize: '9px', color: tokens.colors.textMuted }}>(edit)</span>
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
                gap: '3px',
                padding: '1px 6px',
                borderRadius: '4px',
                fontSize: '10.5px',
                fontWeight: 600,
                background: 'rgba(16, 185, 129, 0.18)',
                color: '#34d399',
                border: '1px solid rgba(16, 185, 129, 0.35)',
                minWidth: '54px',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              <IconWriting size={11} />
              <span>Text</span>
            </span>

            <button
              type="button"
              onClick={onStartEditStep}
              title="Click to edit text snippet"
              style={{
                background: 'rgba(255, 255, 255, 0.06)',
                border: '1px dashed rgba(255, 255, 255, 0.22)',
                borderRadius: '4px',
                padding: '2px 7px',
                fontSize: '11.5px',
                color: tokens.colors.textPrimary,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '5px',
                maxWidth: '220px',
                overflow: 'hidden',
                textAlign: 'left',
              }}
            >
              <span
                style={{
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                "{step.text}"
              </span>
              <IconEdit size={11} style={{ opacity: 0.6, flexShrink: 0 }} />
            </button>
          </>
        );

      case 'RunCommand':
        return (
          <>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '3px',
                padding: '1px 6px',
                borderRadius: '4px',
                fontSize: '10.5px',
                fontWeight: 600,
                background: 'rgba(236, 72, 153, 0.18)',
                color: '#f472b6',
                border: '1px solid rgba(236, 72, 153, 0.35)',
                minWidth: '54px',
                justifyContent: 'center',
                flexShrink: 0,
              }}
            >
              <IconTerminal2 size={11} />
              <span>Cmd</span>
            </span>

            <button
              type="button"
              onClick={onStartEditStep}
              title="Click to edit command script"
              style={{
                background: 'rgba(255, 255, 255, 0.06)',
                border: '1px dashed rgba(255, 255, 255, 0.22)',
                borderRadius: '4px',
                padding: '2px 7px',
                fontSize: '11px',
                fontFamily: tokens.typography.fontMono,
                color: tokens.colors.textPrimary,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '5px',
                maxWidth: '220px',
                overflow: 'hidden',
                textAlign: 'left',
              }}
            >
              <span
                style={{
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap',
                }}
              >
                {step.command}
              </span>
              <IconEdit size={11} style={{ opacity: 0.6, flexShrink: 0 }} />
            </button>
          </>
        );
    }
  };

  return (
    <div
      data-step-index={index}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '4px 8px',
        borderRadius: '5px',
        background: isDragging ? 'rgba(88, 101, 242, 0.08)' : 'rgba(255, 255, 255, 0.03)',
        border: isDragging
          ? '1px dashed rgba(88, 101, 242, 0.45)'
          : '1px solid rgba(255, 255, 255, 0.06)',
        opacity: isDragging ? 0.35 : 1,
        gap: '6px',
        transition: 'opacity 0.1s ease, border-color 0.1s ease',
        userSelect: isDragging ? 'none' : 'auto',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1, minWidth: 0 }}>
        {/* Drag handle using Pointer Capture */}
        <div
          onPointerDown={(e) => {
            if (!canDrag || isEditingDuration) return;
            onGripPointerDown?.(e as unknown as PointerEvent, index);
          }}
          onPointerMove={(e) => onGripPointerMove?.(e as unknown as PointerEvent)}
          onPointerUp={(e) => onGripPointerUp?.(e as unknown as PointerEvent)}
          onPointerCancel={(e) => onGripPointerCancel?.(e as unknown as PointerEvent)}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            cursor: canDrag ? (isDragging ? 'grabbing' : 'grab') : 'default',
            color: isDragging ? '#818cf8' : tokens.colors.textMuted,
            opacity: canDrag ? 0.7 : 0.2,
            padding: '0 2px',
            userSelect: 'none',
            touchAction: 'none',
            flexShrink: 0,
          }}
          title={canDrag ? 'Drag to reorder' : undefined}
        >
          <IconGripVertical size={13} />
        </div>

        <span
          style={{
            fontSize: '10px',
            fontFamily: 'monospace',
            color: tokens.colors.textMuted,
            minWidth: '16px',
            userSelect: 'none',
          }}
        >
          {index + 1}.
        </span>

        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            flexWrap: 'wrap',
            minWidth: 0,
          }}
        >
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
          padding: '2px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '3px',
          flexShrink: 0,
        }}
      >
        <IconX size={13} />
      </button>
    </div>
  );
}
