import { useSignal } from '@preact/signals';
import { useEffect, useRef } from 'preact/hooks';
import type { MacroStep } from '../../../types.ts';
import { normalizeKeyName } from './keyUtils.ts';

export function useMacroSequence(initialSteps: MacroStep[] = []) {
  const steps = useSignal<MacroStep[]>([...initialSteps]);
  const isRecording = useSignal(false);
  const manualKeyInput = useSignal('');
  const manualTextInput = useSignal('');
  const manualCommandInput = useSignal('');
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

  const clearSteps = () => {
    steps.value = [];
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

  const addManualCommand = () => {
    const command = manualCommandInput.value.trim();
    if (!command) return;
    steps.value = [...steps.value, { type: 'RunCommand', command }];
    manualCommandInput.value = '';
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

  const cancelEditDuration = () => {
    editingDurationIndex.value = null;
  };

  const updateStepTextOrCommand = (index: number, textOrCommand: string) => {
    const updated = [...steps.value];
    const targetStep = updated[index];
    if (!targetStep) return;

    if (targetStep.type === 'TypeText') {
      updated[index] = { type: 'TypeText', text: textOrCommand };
      steps.value = updated;
    } else if (targetStep.type === 'RunCommand') {
      updated[index] = { type: 'RunCommand', command: textOrCommand.trim() };
      steps.value = updated;
    }
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

  return {
    steps,
    isRecording,
    manualKeyInput,
    manualTextInput,
    manualCommandInput,
    editingDurationIndex,
    editingDurationValue,
    draggingIndex,
    targetInsertionIndex,
    listScrollRef,
    toggleRecording,
    clearSteps,
    removeStep,
    addManualKey,
    addManualDelay,
    addManualText,
    addManualCommand,
    startEditDuration,
    saveEditDuration,
    cancelEditDuration,
    updateStepTextOrCommand,
    handleGripPointerDown,
    handleGripPointerMove,
    handleGripPointerUp,
    handleGripPointerCancel,
  };
}
