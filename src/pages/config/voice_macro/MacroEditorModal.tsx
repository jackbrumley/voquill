import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import {
  IconCircleDot,
  IconCheck,
  IconPlus,
  IconTrash,
} from '@tabler/icons-preact';
import { Modal } from '../../../components/Modal.tsx';
import { Button } from '../../../components/Button.tsx';
import type { MacroStep, VoiceMacroCommand } from '../../../types.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { tokens } from '../../../design-tokens.ts';
import { normalizeKeyName } from './keyUtils.ts';
import { MacroStepRow } from './MacroStepRow.tsx';

interface MacroEditorModalProps {
  initialCommand: VoiceMacroCommand | null;
  onSave: (phrase: string, steps: MacroStep[]) => void;
  onClose: () => void;
}

export function MacroEditorModal({
  initialCommand,
  onSave,
  onClose,
}: MacroEditorModalProps) {
  const phrase = useSignal(initialCommand ? initialCommand.phrase : '');
  const steps = useSignal<MacroStep[]>(
    initialCommand && initialCommand.steps ? [...initialCommand.steps] : []
  );
  const isRecording = useSignal(false);
  const manualKeyInput = useSignal('');
  const manualTextInput = useSignal('');
  const editingDelayIndex = useSignal<number | null>(null);
  const editingDelayValue = useSignal('');

  const lastEventTime = useSignal<number>(0);
  const pressedKeysDownTime = useSignal<Record<string, number>>({});

  const appendStepWithDelay = (step: MacroStep) => {
    const now = Date.now();
    const updated = [...steps.value];
    if (lastEventTime.value > 0) {
      const delayMs = now - lastEventTime.value;
      if (delayMs >= 35 && updated.length > 0) {
        updated.push({ type: 'Delay', duration_ms: Math.min(delayMs, 5000) });
      }
    }
    updated.push(step);
    steps.value = updated;
    lastEventTime.value = now;
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!isRecording.value) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      isRecording.value = false;
      return;
    }

    const keyName = normalizeKeyName(e);
    const isModifier = ['Ctrl', 'Shift', 'Alt', 'Super'].includes(keyName);

    if (isModifier) {
      if (!pressedKeysDownTime.value[keyName]) {
        pressedKeysDownTime.value = { ...pressedKeysDownTime.value, [keyName]: Date.now() };
        appendStepWithDelay({ type: 'KeyDown', key: keyName });
      }
    } else if (!pressedKeysDownTime.value[keyName]) {
      pressedKeysDownTime.value = { ...pressedKeysDownTime.value, [keyName]: Date.now() };
    }
  };

  const handleKeyUp = (e: KeyboardEvent) => {
    if (!isRecording.value) return;

    e.preventDefault();
    e.stopPropagation();

    const keyName = normalizeKeyName(e);
    const isModifier = ['Ctrl', 'Shift', 'Alt', 'Super'].includes(keyName);
    const downTime = pressedKeysDownTime.value[keyName] || Date.now();

    const updatedPresses = { ...pressedKeysDownTime.value };
    delete updatedPresses[keyName];
    pressedKeysDownTime.value = updatedPresses;

    if (isModifier) {
      appendStepWithDelay({ type: 'KeyUp', key: keyName });
    } else {
      const holdDuration = Math.min(Math.max(Date.now() - downTime, 25), 2000);
      appendStepWithDelay({ type: 'KeyPress', key: keyName, hold_ms: holdDuration });
    }
  };

  useEffect(() => {
    if (!isRecording.value) return;

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [isRecording.value]);

  const toggleRecording = () => {
    if (isRecording.value) {
      isRecording.value = false;
    } else {
      isRecording.value = true;
      lastEventTime.value = 0;
      pressedKeysDownTime.value = {};
    }
  };

  const removeStep = (index: number) => {
    const updated = [...steps.value];
    updated.splice(index, 1);
    steps.value = updated;
  };

  const addManualKey = (type: 'KeyPress' | 'KeyDown' | 'KeyUp') => {
    const key = manualKeyInput.value.trim();
    if (!key) return;
    if (type === 'KeyPress') {
      steps.value = [...steps.value, { type: 'KeyPress', key, hold_ms: 50 }];
    } else if (type === 'KeyDown') {
      steps.value = [...steps.value, { type: 'KeyDown', key }];
    } else if (type === 'KeyUp') {
      steps.value = [...steps.value, { type: 'KeyUp', key }];
    }
    manualKeyInput.value = '';
  };

  const addManualDelay = () => {
    steps.value = [...steps.value, { type: 'Delay', duration_ms: 100 }];
  };

  const addManualText = () => {
    const text = manualTextInput.value.trim();
    if (!text) return;
    steps.value = [...steps.value, { type: 'TypeText', text }];
    manualTextInput.value = '';
  };

  const startEditDelay = (index: number, currentMs: number) => {
    editingDelayIndex.value = index;
    editingDelayValue.value = currentMs.toString();
  };

  const saveEditDelay = (index: number) => {
    const num = parseInt(editingDelayValue.value, 10);
    if (!isNaN(num) && num >= 0) {
      const updated = [...steps.value];
      updated[index] = { type: 'Delay', duration_ms: num };
      steps.value = updated;
    }
    editingDelayIndex.value = null;
  };

  const handleSave = () => {
    const cleanPhrase = phrase.value.trim().toLowerCase();
    if (!cleanPhrase || steps.value.length === 0) return;
    onSave(cleanPhrase, steps.value);
  };

  const isSaveDisabled = !phrase.value.trim() || steps.value.length === 0 || isRecording.value;

  return (
    <Modal
      title={initialCommand ? 'Edit Voice Macro' : 'Create Voice Macro'}
      onClose={onClose}
      maxWidth="620px"
      footer={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '8px', width: '100%' }}>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="configAction"
            onClick={handleSave}
            disabled={isSaveDisabled}
            style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
          >
            <IconCheck size={14} />
            <span>{initialCommand ? 'Save Changes' : 'Create Macro'}</span>
          </Button>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
        {/* Spoken Phrase Field */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <label style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textPrimary, fontWeight: 600 }}>
            Spoken Voice Command
          </label>
          <input
            type="text"
            value={phrase.value}
            onInput={(e) => {
              phrase.value = (e.target as HTMLInputElement).value;
            }}
            placeholder="e.g. call airstrike, ultimate ability, open map"
            autoFocus={!initialCommand}
            style={{ ...inputBaseStyle, width: '100%' }}
          />
          <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
            The exact words you will speak to trigger this automation sequence.
          </span>
        </div>

        {/* Live Sequence Recorder Controls */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '10px 12px',
            borderRadius: '8px',
            background: isRecording.value ? 'rgba(239, 68, 68, 0.12)' : 'rgba(255, 255, 255, 0.04)',
            border: isRecording.value ? '1px solid rgba(239, 68, 68, 0.45)' : '1px solid rgba(255, 255, 255, 0.08)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Button
              variant="configAction"
              onClick={toggleRecording}
              style={
                isRecording.value
                  ? {
                      display: 'flex',
                      alignItems: 'center',
                      gap: '6px',
                      borderColor: '#ef4444',
                      color: '#f87171',
                      background: 'rgba(239, 68, 68, 0.2)',
                    }
                  : { display: 'flex', alignItems: 'center', gap: '6px' }
              }
            >
              {isRecording.value ? (
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

            {steps.value.length > 0 && !isRecording.value && (
              <Button
                variant="ghost"
                onClick={() => {
                  steps.value = [];
                }}
                style={{ display: 'flex', alignItems: 'center', gap: '4px', color: tokens.colors.textMuted }}
              >
                <IconTrash size={13} />
                <span>Clear All</span>
              </Button>
            )}
          </div>

          <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
            {steps.value.length} {steps.value.length === 1 ? 'action' : 'actions'}
          </span>
        </div>

        {/* Live Recording Active Banner */}
        {isRecording.value && (
          <div
            style={{
              padding: '8px 12px',
              borderRadius: '6px',
              background: 'rgba(239, 68, 68, 0.15)',
              border: '1px solid rgba(239, 68, 68, 0.3)',
              fontSize: '11px',
              color: '#fca5a5',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
            }}
          >
            <span>
              🔴 <strong>Recording Active:</strong> Press keys, modifiers, and combos. Delays and hold times are captured in real-time. (Press <strong>Esc</strong> to finish)
            </span>
          </div>
        )}

        {/* Step-by-Step Vertical List */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 2px' }}>
            <span style={{ fontSize: '12px', fontWeight: 600, color: tokens.colors.textSecondary }}>
              Execution Steps (Top to Bottom)
            </span>
          </div>

          <div
            style={{
              maxHeight: '260px',
              minHeight: '80px',
              overflowY: 'auto',
              scrollbarGutter: 'stable',
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              padding: '8px',
              borderRadius: '8px',
              background: 'rgba(0, 0, 0, 0.25)',
              border: '1px solid rgba(255, 255, 255, 0.08)',
            }}
          >
            {steps.value.length === 0 ? (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  height: '70px',
                  fontSize: tokens.typography.sizeXs,
                  color: tokens.colors.textMuted,
                  textAlign: 'center',
                }}
              >
                No actions added yet. Click 'Record Sequence' or add steps manually below.
              </div>
            ) : (
              steps.value.map((step, idx) => (
                <MacroStepRow
                  key={idx}
                  index={idx}
                  step={step}
                  onRemove={() => removeStep(idx)}
                  isEditingDelay={editingDelayIndex.value === idx}
                  editingDelayValue={editingDelayValue.value}
                  onStartEditDelay={
                    step.type === 'Delay' ? () => startEditDelay(idx, step.duration_ms) : undefined
                  }
                  onDelayInputChange={(val) => {
                    editingDelayValue.value = val;
                  }}
                  onSaveDelay={() => saveEditDelay(idx)}
                />
              ))
            )}
          </div>
        </div>

        {/* Manual Step Adders */}
        {!isRecording.value && (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '8px',
              padding: '10px 12px',
              borderRadius: '8px',
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid rgba(255, 255, 255, 0.06)',
            }}
          >
            <span style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted }}>
              MANUAL STEP BUILDER
            </span>

            {/* Key Action Adders */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' }}>
              <input
                type="text"
                value={manualKeyInput.value}
                onInput={(e) => {
                  manualKeyInput.value = (e.target as HTMLInputElement).value;
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    addManualKey('KeyPress');
                  }
                }}
                placeholder="Key (e.g. F3, W, Ctrl)"
                style={{ ...inputBaseStyle, width: '130px', padding: '4px 8px', fontSize: '12px' }}
              />
              <Button
                variant="configAction"
                onClick={() => addManualKey('KeyPress')}
                style={{ padding: '4px 8px', fontSize: '11px' }}
              >
                + Tap
              </Button>
              <Button
                variant="configAction"
                onClick={() => addManualKey('KeyDown')}
                style={{ padding: '4px 8px', fontSize: '11px' }}
              >
                + Hold
              </Button>
              <Button
                variant="configAction"
                onClick={() => addManualKey('KeyUp')}
                style={{ padding: '4px 8px', fontSize: '11px' }}
              >
                + Release
              </Button>
              <Button
                variant="configAction"
                onClick={addManualDelay}
                style={{ padding: '4px 8px', fontSize: '11px' }}
              >
                + 100ms Delay
              </Button>
            </div>

            {/* Type Text Adder */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <input
                type="text"
                value={manualTextInput.value}
                onInput={(e) => {
                  manualTextInput.value = (e.target as HTMLInputElement).value;
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    addManualText();
                  }
                }}
                placeholder="Type text / chat phrase..."
                style={{ ...inputBaseStyle, flex: 1, padding: '4px 8px', fontSize: '12px' }}
              />
              <Button
                variant="configAction"
                onClick={addManualText}
                style={{ padding: '4px 8px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}
              >
                <IconPlus size={12} />
                <span>Add Text</span>
              </Button>
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
