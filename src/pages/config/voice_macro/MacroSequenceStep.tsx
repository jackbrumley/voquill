import { Fragment } from 'preact';
import { useSignal } from '@preact/signals';
import {
  IconCircleFilled,
  IconCheck,
  IconTrash,
} from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { tokens } from '../../../design-tokens.ts';
import { MacroStepRow } from './MacroStepRow.tsx';
import { MacroManualActionBar } from './MacroManualActionBar.tsx';
import { MacroStepDialog } from './MacroStepDialog.tsx';
import type { useMacroSequence } from './useMacroSequence.ts';

interface MacroSequenceStepProps {
  sequence: ReturnType<typeof useMacroSequence>;
}

function DropInsertionLine() {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        height: '8px',
        margin: '-2px 0',
        padding: '0 4px',
        zIndex: 10,
        position: 'relative',
      }}
    >
      <div
        style={{
          width: '6px',
          height: '6px',
          borderRadius: '50%',
          background: '#818cf8',
          boxShadow: '0 0 8px rgba(129, 140, 248, 0.9)',
          flexShrink: 0,
        }}
      />
      <div
        style={{
          flex: 1,
          height: '2px',
          borderRadius: '2px',
          background:
            'linear-gradient(90deg, #818cf8 0%, #6366f1 50%, rgba(99, 102, 241, 0.25) 100%)',
          boxShadow: '0 0 6px rgba(99, 102, 241, 0.7)',
        }}
      />
    </div>
  );
}

export function MacroSequenceStep({ sequence }: MacroSequenceStepProps) {
  const dialogStepIndex = useSignal<number | null>(null);
  const dialogStepType = useSignal<'TypeText' | 'RunCommand'>('TypeText');
  const dialogInitialValue = useSignal<string>('');

  const {
    steps,
    isRecording,
    editingDurationIndex,
    editingDurationValue,
    draggingIndex,
    targetInsertionIndex,
    listScrollRef,
    toggleRecording,
    clearSteps,
    removeStep,
    startEditDuration,
    saveEditDuration,
    cancelEditDuration,
    updateStepTextOrCommand,
    handleGripPointerDown,
    handleGripPointerMove,
    handleGripPointerUp,
    handleGripPointerCancel,
  } = sequence;

  const handleOpenStepDialog = (
    index: number,
    type: 'TypeText' | 'RunCommand',
    value: string
  ) => {
    dialogStepIndex.value = index;
    dialogStepType.value = type;
    dialogInitialValue.value = value;
  };

  const handleSaveStepDialog = (newValue: string) => {
    if (dialogStepIndex.value !== null) {
      updateStepTextOrCommand(dialogStepIndex.value, newValue);
    }
    dialogStepIndex.value = null;
  };

  const handleCancelStepDialog = () => {
    dialogStepIndex.value = null;
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
        flex: 1,
        minHeight: 0,
        height: '100%',
        position: 'relative',
      }}
    >
      {/* Top Recorder Control Bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 10px',
          borderRadius: '7px',
          background: isRecording.value ? 'rgba(239, 68, 68, 0.15)' : 'rgba(255, 255, 255, 0.03)',
          border: isRecording.value
            ? '1px solid rgba(239, 68, 68, 0.5)'
            : '1px solid rgba(255, 255, 255, 0.08)',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Button
            variant={isRecording.value ? 'primary' : 'configAction'}
            onClick={toggleRecording}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '6px 12px',
              fontSize: '12px',
              fontWeight: 600,
              ...(isRecording.value
                ? {
                    background: 'rgba(239, 68, 68, 0.3)',
                    borderColor: '#ef4444',
                    color: '#fca5a5',
                  }
                : {}),
            }}
          >
            {isRecording.value ? (
              <>
                <IconCheck size={14} />
                <span>Done Recording</span>
              </>
            ) : (
              <>
                <IconCircleFilled size={12} color="#ef4444" />
                <span>Record Sequence</span>
              </>
            )}
          </Button>

          {steps.value.length > 0 && !isRecording.value && (
            <Button
              variant="ghost"
              onClick={clearSteps}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '3px',
                padding: '5px 8px',
                fontSize: '11.5px',
                color: tokens.colors.textMuted,
              }}
            >
              <IconTrash size={13} />
              <span>Clear All</span>
            </Button>
          )}
        </div>

        <span
          style={{
            fontSize: '11px',
            fontWeight: 600,
            color: steps.value.length > 0 ? '#a5b4fc' : tokens.colors.textMuted,
            background: steps.value.length > 0 ? 'rgba(99, 102, 241, 0.15)' : 'transparent',
            padding: '2px 8px',
            borderRadius: '12px',
          }}
        >
          {steps.value.length} {steps.value.length === 1 ? 'step' : 'steps'}
        </span>
      </div>

      {/* Live Recording Active Banner */}
      {isRecording.value && (
        <div
          style={{
            padding: '8px 12px',
            borderRadius: '6px',
            background: 'rgba(239, 68, 68, 0.1)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            fontSize: '11px',
            color: '#fca5a5',
            flexShrink: 0,
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span
              style={{
                display: 'inline-block',
                width: '8px',
                height: '8px',
                borderRadius: '50%',
                background: '#ef4444',
                boxShadow: '0 0 8px #ef4444',
              }}
            />
            <span>Listening for keystrokes... press keys on your keyboard</span>
          </div>
          <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
            Press Esc to finish
          </span>
        </div>
      )}

      {/* Execution Sequence Vertical List - Hero container taking maximum flex */}
      <div
        ref={listScrollRef}
        style={{
          flex: 1,
          minHeight: 0,
          overflowY: 'auto',
          scrollbarGutter: 'stable',
          display: 'flex',
          flexDirection: 'column',
          gap: '4px',
          padding: '6px',
          borderRadius: '7px',
          background: 'rgba(0, 0, 0, 0.35)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}
      >
        {steps.value.length === 0 ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              minHeight: '120px',
              padding: '16px',
              textAlign: 'center',
              gap: '6px',
            }}
          >
            <span style={{ fontSize: '12px', fontWeight: 600, color: tokens.colors.textSecondary }}>
              No actions in sequence yet
            </span>
            <span
              style={{
                fontSize: '11px',
                color: tokens.colors.textMuted,
                maxWidth: '280px',
                lineHeight: 1.4,
              }}
            >
              Click <strong>"Record Sequence"</strong> above to record live keystrokes, or insert
              actions manually below.
            </span>
          </div>
        ) : (
          steps.value.map((step, idx) => {
            const currentDuration =
              step.type === 'Delay'
                ? step.duration_ms
                : step.type === 'KeyPress'
                  ? step.hold_ms || 50
                  : 0;
            const isDurationEditable = step.type === 'Delay' || step.type === 'KeyPress';

            let onStartEditStep: (() => void) | undefined;
            if (step.type === 'TypeText') {
              onStartEditStep = () => handleOpenStepDialog(idx, 'TypeText', step.text);
            } else if (step.type === 'RunCommand') {
              onStartEditStep = () => handleOpenStepDialog(idx, 'RunCommand', step.command);
            }

            const isItemDragging = draggingIndex.value === idx;
            const isLast = idx === steps.value.length - 1;

            return (
              <Fragment key={idx}>
                {targetInsertionIndex.value === idx && <DropInsertionLine />}
                <MacroStepRow
                  index={idx}
                  step={step}
                  onRemove={() => removeStep(idx)}
                  isEditingDuration={editingDurationIndex.value === idx}
                  editingDurationValue={editingDurationValue.value}
                  onStartEditDuration={
                    isDurationEditable
                      ? () => startEditDuration(idx, currentDuration)
                      : undefined
                  }
                  onDurationInputChange={(val) => {
                    editingDurationValue.value = val;
                  }}
                  onSaveDuration={() => saveEditDuration(idx)}
                  onCancelDuration={cancelEditDuration}
                  onStartEditStep={onStartEditStep}
                  canDrag={!isRecording.value}
                  isDragging={isItemDragging}
                  onGripPointerDown={(e) => handleGripPointerDown(e, idx)}
                  onGripPointerMove={handleGripPointerMove}
                  onGripPointerUp={handleGripPointerUp}
                  onGripPointerCancel={handleGripPointerCancel}
                />
                {isLast && targetInsertionIndex.value === steps.value.length && (
                  <DropInsertionLine />
                )}
              </Fragment>
            );
          })
        )}
      </div>

      {/* Manual Action Adder */}
      {!isRecording.value && <MacroManualActionBar sequence={sequence} />}

      {/* Dedicated Step Editor Popover Dialog for Text & Command */}
      {dialogStepIndex.value !== null && (
        <MacroStepDialog
          stepIndex={dialogStepIndex.value}
          stepType={dialogStepType.value}
          initialValue={dialogInitialValue.value}
          onSave={handleSaveStepDialog}
          onCancel={handleCancelStepDialog}
        />
      )}
    </div>
  );
}
