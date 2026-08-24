import { Signal } from '@preact/signals';
import {
  IconPlus,
  IconCircleDot,
  IconCheck,
} from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import type { MacroStep } from '../../../types.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { tokens } from '../../../design-tokens.ts';
import { MacroStepChip } from './MacroStepChip.tsx';

interface MacroSequenceBuilderProps {
  newPhrase: Signal<string>;
  newSteps: Signal<MacroStep[]>;
  editingMacroId: Signal<string | null>;
  isRecordingSequence: Signal<boolean>;
  manualKeyInput: Signal<string>;
  editingDelayIndex: Signal<number | null>;
  editingDelayValue: Signal<string>;
  onSave: () => void;
  onCancelEdit: () => void;
  onToggleRecord: () => void;
  onRemoveStep: (index: number) => void;
  onAddManualKey: (type: 'KeyPress' | 'KeyDown' | 'KeyUp') => void;
  onAddManualDelay: () => void;
  onStartEditDelay: (index: number, ms: number) => void;
  onSaveEditDelay: (index: number) => void;
}

export function MacroSequenceBuilder({
  newPhrase,
  newSteps,
  editingMacroId,
  isRecordingSequence,
  manualKeyInput,
  editingDelayIndex,
  editingDelayValue,
  onSave,
  onCancelEdit,
  onToggleRecord,
  onRemoveStep,
  onAddManualKey,
  onAddManualDelay,
  onStartEditDelay,
  onSaveEditDelay,
}: MacroSequenceBuilderProps) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: tokens.spacing.sm,
        padding: tokens.spacing.sm,
        borderRadius: '8px',
        background: 'rgba(255, 255, 255, 0.04)',
        border: '1px solid rgba(255, 255, 255, 0.08)',
      }}
    >
      <div style={{ display: 'flex', gap: tokens.spacing.xs, width: '100%' }}>
        <input
          type="text"
          value={newPhrase.value}
          onInput={(e) => {
            newPhrase.value = (e.target as HTMLInputElement).value;
          }}
          placeholder="Spoken command (e.g. call airstrike)"
          style={{ ...inputBaseStyle, flex: 1 }}
        />
        <div style={{ display: 'flex', gap: '6px' }}>
          <Button
            variant="configAction"
            onClick={onSave}
            disabled={!newPhrase.value.trim() || newSteps.value.length === 0}
            style={{ display: 'flex', alignItems: 'center', gap: '4px' }}
          >
            {editingMacroId.value ? (
              <>
                <IconCheck size={14} />
                <span>Update Macro</span>
              </>
            ) : (
              <>
                <IconPlus size={14} />
                <span>Save Macro</span>
              </>
            )}
          </Button>
          {editingMacroId.value && (
            <Button variant="ghost" onClick={onCancelEdit}>
              Cancel
            </Button>
          )}
        </div>
      </div>

      {/* Sequence Step Timeline */}
      <div
        style={{
          minHeight: '44px',
          padding: '8px',
          borderRadius: '6px',
          background: isRecordingSequence.value
            ? 'rgba(239, 68, 68, 0.08)'
            : 'rgba(0, 0, 0, 0.25)',
          border: isRecordingSequence.value
            ? '1px solid rgba(239, 68, 68, 0.4)'
            : '1px solid rgba(255, 255, 255, 0.06)',
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          flexWrap: 'wrap',
        }}
      >
        {newSteps.value.length === 0 && !isRecordingSequence.value && (
          <span style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>
            Click 'Record Sequence' below or use the manual buttons to build steps.
          </span>
        )}

        {newSteps.value.map((step, idx) => (
          <div key={idx} style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
            <MacroStepChip
              step={step}
              onRemove={() => onRemoveStep(idx)}
              isEditingDelay={editingDelayIndex.value === idx}
              editingDelayValue={editingDelayValue.value}
              onStartEditDelay={
                step.type === 'Delay' ? () => onStartEditDelay(idx, step.duration_ms) : undefined
              }
              onDelayInputChange={(val) => {
                editingDelayValue.value = val;
              }}
              onSaveDelay={() => onSaveEditDelay(idx)}
            />
            {idx < newSteps.value.length - 1 && (
              <span style={{ color: tokens.colors.textMuted, fontSize: '10px' }}>➔</span>
            )}
          </div>
        ))}
      </div>

      {/* Controls & Mode Bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: '6px',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Button
            variant="configAction"
            onClick={onToggleRecord}
            style={
              isRecordingSequence.value
                ? {
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    borderColor: '#ef4444',
                    color: '#f87171',
                  }
                : { display: 'flex', alignItems: 'center', gap: '6px' }
            }
          >
            {isRecordingSequence.value ? (
              <>
                <IconCheck size={14} />
                <span>Done Recording</span>
              </>
            ) : (
              <>
                <IconCircleDot size={14} color="#ef4444" />
                <span>Record Sequence</span>
              </>
            )}
          </Button>

          {newSteps.value.length > 0 && (
            <Button
              variant="configAction"
              onClick={() => {
                newSteps.value = [];
              }}
            >
              Clear
            </Button>
          )}
        </div>

        {/* Manual Add Inputs */}
        {!isRecordingSequence.value && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
            <input
              type="text"
              value={manualKeyInput.value}
              onInput={(e) => {
                manualKeyInput.value = (e.target as HTMLInputElement).value;
              }}
              placeholder="Key (e.g. F3, Ctrl)"
              style={{ ...inputBaseStyle, width: '110px', padding: '4px 8px', fontSize: '12px' }}
            />
            <Button
              variant="configAction"
              onClick={() => onAddManualKey('KeyPress')}
              style={{ padding: '4px 8px', fontSize: '11px' }}
            >
              + Tap
            </Button>
            <Button
              variant="configAction"
              onClick={() => onAddManualKey('KeyDown')}
              style={{ padding: '4px 8px', fontSize: '11px' }}
            >
              + Hold
            </Button>
            <Button
              variant="configAction"
              onClick={() => onAddManualKey('KeyUp')}
              style={{ padding: '4px 8px', fontSize: '11px' }}
            >
              + Release
            </Button>
            <Button
              variant="configAction"
              onClick={onAddManualDelay}
              style={{ padding: '4px 8px', fontSize: '11px' }}
            >
              + Delay
            </Button>
          </div>
        )}
      </div>

      {isRecordingSequence.value && (
        <div
          style={{
            fontSize: '11px',
            color: '#f87171',
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '4px 0',
          }}
        >
          <span>
            ● Live Recording: Press keys and modifiers. Delays and hold times are captured in
            real-time. (Press Esc to finish)
          </span>
        </div>
      )}
    </div>
  );
}
