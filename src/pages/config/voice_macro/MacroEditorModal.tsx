import { Fragment } from 'preact';
import { useSignal } from '@preact/signals';
import { useEffect, useRef } from 'preact/hooks';
import { IconCheck, IconTrash } from '@tabler/icons-preact';
import { confirm } from '@tauri-apps/plugin-dialog';
import { Modal } from '../../../components/Modal.tsx';
import { Button } from '../../../components/Button.tsx';
import type { MacroStep, VoiceMacroCommand } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { normalizeKeyName } from './keyUtils.ts';
import { MacroStepRow } from './MacroStepRow.tsx';
import { MacroPhrasesEditor } from './MacroPhrasesEditor.tsx';
import { MacroRecorderToolbar } from './MacroRecorderToolbar.tsx';
import { MacroManualAdders } from './MacroManualAdders.tsx';

interface MacroEditorModalProps {
  initialCommand: VoiceMacroCommand | null;
  onSave: (phrase: string, phrases: string[], steps: MacroStep[]) => void;
  onDelete?: () => void;
  onClose: () => void;
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

export function MacroEditorModal({
  initialCommand,
  onSave,
  onDelete,
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
  const editingDurationIndex = useSignal<number | null>(null);
  const editingDurationValue = useSignal('');

  const lastEventTime = useSignal<number>(0);
  const pressedKeysDownTime = useSignal<Record<string, number>>({});

  // Pointer drag and drop state: records slot insertion index between steps (0 to steps.length)
  const draggingIndex = useSignal<number | null>(null);
  const targetInsertionIndex = useSignal<number | null>(null);
  const listScrollRef = useRef<HTMLDivElement>(null);

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

  const startEditDuration = (index: number, currentMs: number) => {
    editingDurationIndex.value = index;
    editingDurationValue.value = currentMs.toString();
  };

  const saveEditDuration = (index: number) => {
    const num = parseInt(editingDurationValue.value, 10);
    if (!isNaN(num) && num >= 0) {
      const updated = [...steps.value];
      const targetStep = updated[index];
      if (targetStep.type === 'Delay') {
        updated[index] = { type: 'Delay', duration_ms: num };
      } else if (targetStep.type === 'KeyPress') {
        updated[index] = { type: 'KeyPress', key: targetStep.key, hold_ms: num };
      }
      steps.value = updated;
    }
    editingDurationIndex.value = null;
  };

  // Pointer drag and drop event handlers
  const handleGripPointerDown = (e: PointerEvent, index: number) => {
    if (e.button !== 0) return;
    e.preventDefault();
    try {
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    } catch {
      // Ignore if setPointerCapture is unsupported
    }
    draggingIndex.value = index;
    targetInsertionIndex.value = null;
  };

  const handleGripPointerMove = (e: PointerEvent) => {
    if (draggingIndex.value === null) return;
    e.preventDefault();

    const elements = document.elementsFromPoint(e.clientX, e.clientY);
    let targetRow: HTMLElement | null = null;
    for (const el of elements) {
      const row = el.closest('[data-step-index]') as HTMLElement | null;
      if (row) {
        targetRow = row;
        break;
      }
    }

    if (targetRow) {
      const targetIndex = parseInt(targetRow.getAttribute('data-step-index') || '-1', 10);
      if (targetIndex >= 0) {
        const rect = targetRow.getBoundingClientRect();
        const isTopHalf = e.clientY < rect.top + rect.height / 2;
        const slotIndex = isTopHalf ? targetIndex : targetIndex + 1;

        const from = draggingIndex.value;
        if (slotIndex === from || slotIndex === from + 1) {
          targetInsertionIndex.value = null;
        } else {
          targetInsertionIndex.value = slotIndex;
        }
      }
    }

    // Auto-scroll list when dragging near top or bottom
    if (listScrollRef.current) {
      const containerRect = listScrollRef.current.getBoundingClientRect();
      if (e.clientY < containerRect.top + 35) {
        listScrollRef.current.scrollTop -= 8;
      } else if (e.clientY > containerRect.bottom - 35) {
        listScrollRef.current.scrollTop += 8;
      }
    }
  };

  const handleGripPointerUp = (e: PointerEvent) => {
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Ignore if releasePointerCapture is unsupported
    }

    const fromIndex = draggingIndex.value;
    const insertSlot = targetInsertionIndex.value;

    if (
      fromIndex !== null &&
      insertSlot !== null &&
      insertSlot !== fromIndex &&
      insertSlot !== fromIndex + 1
    ) {
      const updated = [...steps.value];
      const [item] = updated.splice(fromIndex, 1);
      const finalIndex = fromIndex < insertSlot ? insertSlot - 1 : insertSlot;
      const boundedIndex = Math.max(0, Math.min(finalIndex, updated.length));
      updated.splice(boundedIndex, 0, item);
      steps.value = updated;
    }

    draggingIndex.value = null;
    targetInsertionIndex.value = null;
  };

  const handleGripPointerCancel = (e: PointerEvent) => {
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Ignore
    }
    draggingIndex.value = null;
    targetInsertionIndex.value = null;
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
  const handleDelete = async () => {
    if (!onDelete) return;
    const targetPhrase = initialCommand?.phrase || phrases.value[0] || 'this macro';
    const shouldDelete = await confirm(
      `Are you sure you want to delete the voice macro "${targetPhrase}"?`,
      {
        title: 'Delete Voice Macro',
        kind: 'warning',
      }
    );
    if (shouldDelete) {
      onDelete();
    }
  };

  const isSaveDisabled = totalPhraseCount === 0 || steps.value.length === 0 || isRecording.value;

  return (
    <Modal
      title={initialCommand ? 'Edit Voice Macro' : 'Create Voice Macro'}
      onClose={onClose}
      fullScreen
      footer={
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            width: '100%',
          }}
        >
          <div>
            {initialCommand && onDelete && (
              <Button
                variant="danger"
                onClick={handleDelete}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  padding: '6px 10px',
                  fontSize: '11.5px',
                }}
              >
                <IconTrash size={13} />
                <span>Delete</span>
              </Button>
            )}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Button
              variant="ghost"
              onClick={onClose}
              style={{ padding: '6px 12px', fontSize: '12px' }}
            >
              Cancel
            </Button>
            <Button
              variant="configAction"
              onClick={handleSave}
              disabled={isSaveDisabled}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                padding: '6px 14px',
                fontSize: '12px',
              }}
            >
              <IconCheck size={14} />
              <span>{initialCommand ? 'Save Macro' : 'Create Macro'}</span>
            </Button>
          </div>
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
        <MacroPhrasesEditor
          phrases={phrases.value}
          phraseInput={phraseInput.value}
          onPhraseInputChange={(val) => {
            phraseInput.value = val;
          }}
          onAddPhrase={addPhrase}
          onRemovePhrase={removePhrase}
          autoFocus={!initialCommand}
        />

        {/* Live Sequence Recorder Controls */}
        <MacroRecorderToolbar
          isRecording={isRecording.value}
          stepCount={steps.value.length}
          onToggleRecording={toggleRecording}
          onClearSteps={() => {
            steps.value = [];
          }}
        />

        {/* Step-by-Step Vertical Timeline - Claims all available vertical space */}
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            flex: 1,
            minHeight: 0,
            gap: '4px',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '0 2px',
              flexShrink: 0,
            }}
          >
            <span
              style={{
                fontSize: '11px',
                fontWeight: 600,
                color: tokens.colors.textMuted,
                textTransform: 'uppercase',
                letterSpacing: '0.4px',
              }}
            >
              Execution Sequence
            </span>
          </div>

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
              steps.value.map((step, idx) => {
                const currentDuration =
                  step.type === 'Delay'
                    ? step.duration_ms
                    : step.type === 'KeyPress'
                      ? step.hold_ms || 50
                      : 0;
                const isEditable = step.type === 'Delay' || step.type === 'KeyPress';
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
                        isEditable ? () => startEditDuration(idx, currentDuration) : undefined
                      }
                      onDurationInputChange={(val) => {
                        editingDurationValue.value = val;
                      }}
                      onSaveDuration={() => saveEditDuration(idx)}
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
        </div>

        {/* Manual Step Adders */}
        {!isRecording.value && (
          <MacroManualAdders
            manualKeyInput={manualKeyInput.value}
            onManualKeyInputChange={(val) => {
              manualKeyInput.value = val;
            }}
            manualTextInput={manualTextInput.value}
            onManualTextInputChange={(val) => {
              manualTextInput.value = val;
            }}
            onAddManualKey={addManualKey}
            onAddManualDelay={addManualDelay}
            onAddManualText={addManualText}
          />
        )}
      </div>
    </Modal>
  );
}
