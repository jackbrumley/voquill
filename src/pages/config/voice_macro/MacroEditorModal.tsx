import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import {
  IconCircleDot,
  IconCheck,
  IconPlus,
  IconTrash,
  IconX,
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
  onSave: (phrase: string, phrases: string[], steps: MacroStep[]) => void;
  onClose: () => void;
}

export function MacroEditorModal({
  initialCommand,
  onSave,
  onClose,
}: MacroEditorModalProps) {
  // Initialize phrase list: primary phrase + any alias phrases
  const initialPhrases: string[] = [];
  if (initialCommand) {
    if (initialCommand.phrase.trim()) {
      initialPhrases.push(initialCommand.phrase.trim().toLowerCase());
    }
    if (initialCommand.phrases) {
      for (const p of initialCommand.phrases) {
        const clean = p.trim().toLowerCase();
        if (clean && !initialPhrases.includes(clean)) {
          initialPhrases.push(clean);
        }
      }
    }
  }

  const phrases = useSignal<string[]>(initialPhrases);
  const phraseInput = useSignal('');
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

  const addPhrase = () => {
    const raw = phraseInput.value.replace(/,/g, '').trim().toLowerCase();
    if (!raw) return;
    if (!phrases.value.includes(raw)) {
      phrases.value = [...phrases.value, raw];
    }
    phraseInput.value = '';
  };

  const removePhrase = (index: number) => {
    const updated = [...phrases.value];
    updated.splice(index, 1);
    phrases.value = updated;
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
    const all = [...phrases.value];
    const pending = phraseInput.value.replace(/,/g, '').trim().toLowerCase();
    if (pending && !all.includes(pending)) {
      all.push(pending);
    }

    if (all.length === 0 || steps.value.length === 0) return;

    const primaryPhrase = all[0];
    const aliasPhrases = all.slice(1);
    onSave(primaryPhrase, aliasPhrases, steps.value);
  };

  const totalPhraseCount = phrases.value.length + (phraseInput.value.trim() ? 1 : 0);
  const isSaveDisabled = totalPhraseCount === 0 || steps.value.length === 0 || isRecording.value;

  return (
    <Modal
      title={initialCommand ? 'Edit Voice Macro' : 'Create Voice Macro'}
      onClose={onClose}
      fullScreen
      footer={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: '8px', width: '100%' }}>
          <Button variant="ghost" onClick={onClose} style={{ padding: '6px 12px', fontSize: '12px' }}>
            Cancel
          </Button>
          <Button
            variant="configAction"
            onClick={handleSave}
            disabled={isSaveDisabled}
            style={{ display: 'flex', alignItems: 'center', gap: '4px', padding: '6px 14px', fontSize: '12px' }}
          >
            <IconCheck size={14} />
            <span>{initialCommand ? 'Save Macro' : 'Create Macro'}</span>
          </Button>
        </div>
      }
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '8px',
          height: '100%',
          minHeight: 0,
        }}
      >
        {/* Multi-Phrase Tag / Alias Field */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '3px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <label style={{ fontSize: '11px', color: tokens.colors.textMuted, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.4px' }}>
              Spoken Trigger Phrases (Aliases)
            </label>
            <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
              Press Enter or comma to add
            </span>
          </div>

          <div style={{ display: 'flex', gap: '4px' }}>
            <input
              type="text"
              value={phraseInput.value}
              onInput={(e) => {
                phraseInput.value = (e.target as HTMLInputElement).value;
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ',') {
                  e.preventDefault();
                  addPhrase();
                }
              }}
              placeholder={phrases.value.length === 0 ? 'e.g. call airstrike (press Enter to add)' : 'Add another alias phrase...'}
              autoFocus={!initialCommand}
              style={{ ...inputBaseStyle, flex: 1, padding: '5px 8px', fontSize: '12px' }}
            />
            <Button
              variant="configAction"
              onClick={addPhrase}
              disabled={!phraseInput.value.trim()}
              style={{ padding: '4px 8px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '3px' }}
            >
              <IconPlus size={12} />
              <span>Add</span>
            </Button>
          </div>

          {/* Rendered Phrase Chips */}
          {phrases.value.length > 0 && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap', marginTop: '2px' }}>
              {phrases.value.map((p, idx) => (
                <span
                  key={idx}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '4px',
                    padding: '2px 6px',
                    borderRadius: '4px',
                    fontSize: '11px',
                    fontWeight: 600,
                    background: idx === 0 ? 'rgba(88, 101, 242, 0.22)' : 'rgba(255, 255, 255, 0.08)',
                    border: idx === 0 ? '1px solid rgba(88, 101, 242, 0.45)' : '1px solid rgba(255, 255, 255, 0.12)',
                    color: idx === 0 ? '#9ba5ff' : tokens.colors.textSecondary,
                  }}
                >
                  <span>"{p}"</span>
                  {idx === 0 && <span style={{ fontSize: '9px', opacity: 0.7 }}>(primary)</span>}
                  <button
                    type="button"
                    onClick={() => removePhrase(idx)}
                    style={{
                      background: 'none',
                      border: 'none',
                      color: tokens.colors.textMuted,
                      cursor: 'pointer',
                      padding: '0 1px',
                      display: 'flex',
                      alignItems: 'center',
                    }}
                  >
                    <IconX size={11} />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Live Sequence Recorder Controls */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '6px 10px',
            borderRadius: '6px',
            background: isRecording.value ? 'rgba(239, 68, 68, 0.12)' : 'rgba(255, 255, 255, 0.04)',
            border: isRecording.value ? '1px solid rgba(239, 68, 68, 0.45)' : '1px solid rgba(255, 255, 255, 0.08)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Button
              variant="configAction"
              onClick={toggleRecording}
              style={
                isRecording.value
                  ? {
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                      padding: '4px 8px',
                      fontSize: '11px',
                      borderColor: '#ef4444',
                      color: '#f87171',
                      background: 'rgba(239, 68, 68, 0.2)',
                    }
                  : { display: 'flex', alignItems: 'center', gap: '4px', padding: '4px 8px', fontSize: '11px' }
              }
            >
              {isRecording.value ? (
                <>
                  <IconCheck size={13} />
                  <span>Done Recording</span>
                </>
              ) : (
                <>
                  <IconCircleDot size={13} color="#ef4444" />
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
                style={{ display: 'flex', alignItems: 'center', gap: '3px', padding: '4px 6px', fontSize: '11px', color: tokens.colors.textMuted }}
              >
                <IconTrash size={12} />
                <span>Clear</span>
              </Button>
            )}
          </div>

          <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
            {steps.value.length} {steps.value.length === 1 ? 'step' : 'steps'}
          </span>
        </div>

        {/* Live Recording Active Banner */}
        {isRecording.value && (
          <div
            style={{
              padding: '6px 10px',
              borderRadius: '5px',
              background: 'rgba(239, 68, 68, 0.15)',
              border: '1px solid rgba(239, 68, 68, 0.3)',
              fontSize: '11px',
              color: '#fca5a5',
              lineHeight: 1.3,
            }}
          >
            🔴 <strong>Recording...</strong> Press keys, modifiers, combos. Delays are captured. Press <strong>Esc</strong> to finish.
          </div>
        )}

        {/* Step-by-Step Vertical Timeline */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            flex: 1,
            minHeight: '120px',
            maxHeight: '220px',
            gap: '4px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 2px' }}>
            <span style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted, textTransform: 'uppercase', letterSpacing: '0.4px' }}>
              Execution Sequence
            </span>
          </div>

          <div
            style={{
              flex: 1,
              overflowY: 'auto',
              scrollbarGutter: 'stable',
              display: 'flex',
              flexDirection: 'column',
              gap: '4px',
              padding: '6px',
              borderRadius: '6px',
              background: 'rgba(0, 0, 0, 0.3)',
              border: '1px solid rgba(255, 255, 255, 0.08)',
            }}
          >
            {steps.value.length === 0 ? (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  height: '100%',
                  minHeight: '60px',
                  fontSize: '11px',
                  color: tokens.colors.textMuted,
                  textAlign: 'center',
                  padding: '12px',
                }}
              >
                No actions added yet. Click 'Record Sequence' or use the buttons below.
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
              gap: '6px',
              padding: '8px',
              borderRadius: '6px',
              background: 'rgba(255, 255, 255, 0.02)',
              border: '1px solid rgba(255, 255, 255, 0.06)',
            }}
          >
            {/* Key Action Adders */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap' }}>
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
                placeholder="Key (F3, W, Ctrl)"
                style={{ ...inputBaseStyle, width: '100px', padding: '3px 6px', fontSize: '11px' }}
              />
              <Button
                variant="configAction"
                onClick={() => addManualKey('KeyPress')}
                style={{ padding: '3px 6px', fontSize: '10.5px' }}
              >
                + Tap
              </Button>
              <Button
                variant="configAction"
                onClick={() => addManualKey('KeyDown')}
                style={{ padding: '3px 6px', fontSize: '10.5px' }}
              >
                + Hold
              </Button>
              <Button
                variant="configAction"
                onClick={() => addManualKey('KeyUp')}
                style={{ padding: '3px 6px', fontSize: '10.5px' }}
              >
                + Rel
              </Button>
              <Button
                variant="configAction"
                onClick={addManualDelay}
                style={{ padding: '3px 6px', fontSize: '10.5px' }}
              >
                + 100ms
              </Button>
            </div>

            {/* Type Text Adder */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
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
                placeholder="Type text string..."
                style={{ ...inputBaseStyle, flex: 1, padding: '3px 6px', fontSize: '11px' }}
              />
              <Button
                variant="configAction"
                onClick={addManualText}
                style={{ padding: '3px 6px', fontSize: '10.5px', display: 'flex', alignItems: 'center', gap: '3px' }}
              >
                <IconPlus size={11} />
                <span>+ Text</span>
              </Button>
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
