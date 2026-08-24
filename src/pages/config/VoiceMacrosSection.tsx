import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import {
  IconTrash,
  IconVolume,
  IconPlayerPlay,
  IconPencil,
} from '@tabler/icons-preact';
import { invoke } from '@tauri-apps/api/core';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { Button } from '../../components/Button.tsx';
import { SliderField } from '../../components/SliderField.tsx';
import type { Config, MacroStep, VoiceMacroCommand } from '../../types.ts';
import { inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import { normalizeKeyName, resolveMacroSteps } from './voice_macro/keyUtils.ts';
import { MacroStepChip } from './voice_macro/MacroStepChip.tsx';
import { MacroSequenceBuilder } from './voice_macro/MacroSequenceBuilder.tsx';

interface VoiceMacrosSectionProps {
  config: Config;
  updateConfig: (
    key: string,
    value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]
  ) => void;
  showToast?: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

export function VoiceMacrosSection({ config, updateConfig, showToast }: VoiceMacrosSectionProps) {
  const newPhrase = useSignal('');
  const newSteps = useSignal<MacroStep[]>([]);
  const editingMacroId = useSignal<string | null>(null);
  const isRecordingSequence = useSignal(false);
  const isPlayingTestSound = useSignal(false);
  const isTestingExecution = useSignal<string | null>(null);

  const lastEventTime = useSignal<number>(0);
  const pressedKeysDownTime = useSignal<Record<string, number>>({});
  const manualKeyInput = useSignal('');
  const editingDelayIndex = useSignal<number | null>(null);
  const editingDelayValue = useSignal('');

  const handleTestSound = async () => {
    isPlayingTestSound.value = true;
    try {
      await invoke('test_voice_macro_sound');
    } catch (e) {
      showToast?.(`Failed to play sound: ${e}`, 'error');
    } finally {
      setTimeout(() => {
        isPlayingTestSound.value = false;
      }, 600);
    }
  };

  const handleTestMacro = async (cmd: VoiceMacroCommand) => {
    isTestingExecution.value = cmd.id;
    const steps = resolveMacroSteps(cmd);
    try {
      await invoke('test_voice_macro_execution', { steps });
      showToast?.(`Executed macro "${cmd.phrase}" (${steps.length} steps)`, 'success');
    } catch (e) {
      showToast?.(`Failed to execute macro: ${e}`, 'error');
    } finally {
      setTimeout(() => {
        isTestingExecution.value = null;
      }, 500);
    }
  };

  const handleStartEdit = (cmd: VoiceMacroCommand) => {
    editingMacroId.value = cmd.id;
    newPhrase.value = cmd.phrase;
    newSteps.value = [...resolveMacroSteps(cmd)];
    isRecordingSequence.value = false;
    showToast?.(`Editing macro "${cmd.phrase}"`, 'info');
  };

  const handleCancelEdit = () => {
    editingMacroId.value = null;
    newPhrase.value = '';
    newSteps.value = [];
    isRecordingSequence.value = false;
  };

  const handleAddCommand = () => {
    const phrase = newPhrase.value.trim().toLowerCase();
    const steps = newSteps.value;

    if (!phrase) {
      showToast?.('Please enter a spoken phrase for this macro.', 'error');
      return;
    }

    if (steps.length === 0) {
      showToast?.('Please add or record at least one macro action step.', 'error');
      return;
    }

    const currentMacros = config.voice_macros || [];

    if (editingMacroId.value) {
      const updated = currentMacros.map((cmd) => {
        if (cmd.id === editingMacroId.value) {
          return {
            ...cmd,
            phrase,
            steps: [...steps],
            key_combination: null,
            hold_ms: null,
            delay_after_ms: null,
          };
        }
        return cmd;
      });
      updateConfig('voice_macros', updated);
      showToast?.(`Updated macro "${phrase}"`, 'success');
    } else {
      const newCommand: VoiceMacroCommand = {
        id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
        phrase,
        steps: [...steps],
        key_combination: null,
        hold_ms: null,
        delay_after_ms: null,
      };
      updateConfig('voice_macros', [...currentMacros, newCommand]);
      showToast?.(`Added macro command "${phrase}"`, 'success');
    }

    newPhrase.value = '';
    newSteps.value = [];
    editingMacroId.value = null;
    isRecordingSequence.value = false;
  };

  const handleDeleteCommand = (id: string) => {
    const currentMacros = config.voice_macros || [];
    const updated = currentMacros.filter((m) => m.id !== id);
    updateConfig('voice_macros', updated);
  };

  const appendStepWithDelay = (step: MacroStep) => {
    const now = Date.now();
    const updated = [...newSteps.value];
    if (lastEventTime.value > 0) {
      const delayMs = now - lastEventTime.value;
      if (delayMs >= 35 && updated.length > 0) {
        updated.push({ type: 'Delay', duration_ms: Math.min(delayMs, 5000) });
      }
    }
    updated.push(step);
    newSteps.value = updated;
    lastEventTime.value = now;
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!isRecordingSequence.value) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      isRecordingSequence.value = false;
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
    if (!isRecordingSequence.value) return;

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
    if (!isRecordingSequence.value) return;

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [isRecordingSequence.value]);

  const removeStep = (index: number) => {
    const updated = [...newSteps.value];
    updated.splice(index, 1);
    newSteps.value = updated;
  };

  const addManualKey = (type: 'KeyPress' | 'KeyDown' | 'KeyUp') => {
    const key = manualKeyInput.value.trim();
    if (!key) return;
    if (type === 'KeyPress') {
      newSteps.value = [...newSteps.value, { type: 'KeyPress', key, hold_ms: 50 }];
    } else if (type === 'KeyDown') {
      newSteps.value = [...newSteps.value, { type: 'KeyDown', key }];
    } else if (type === 'KeyUp') {
      newSteps.value = [...newSteps.value, { type: 'KeyUp', key }];
    }
    manualKeyInput.value = '';
  };

  const addManualDelay = () => {
    newSteps.value = [...newSteps.value, { type: 'Delay', duration_ms: 100 }];
  };

  const startEditDelay = (index: number, currentMs: number) => {
    editingDelayIndex.value = index;
    editingDelayValue.value = currentMs.toString();
  };

  const saveEditDelay = (index: number) => {
    const num = parseInt(editingDelayValue.value, 10);
    if (!isNaN(num) && num >= 0) {
      const updated = [...newSteps.value];
      updated[index] = { type: 'Delay', duration_ms: num };
      newSteps.value = updated;
    }
    editingDelayIndex.value = null;
  };

  const macros = config.voice_macros || [];

  return (
    <>
      <ConfigField
        label="Always-Listening Voice Macros"
        description="Continuously listen in the background for configured command phrases and execute the corresponding multi-step macro sequences."
      >
        <div style={{ display: 'flex', alignItems: 'center', height: '100%' }}>
          <Switch
            name="Always-Listening Voice Macros"
            checked={config.voice_macros_enabled}
            onChange={(checked) => updateConfig('voice_macros_enabled', checked)}
          />
        </div>
      </ConfigField>

      <ConfigField
        label="Trigger Prefix (Optional)"
        description="Require a prefix word before the command (e.g. 'Computer', 'Voquill') to prevent accidental triggers in game chat or Discord. Leave empty to trigger commands directly."
      >
        <input
          type="text"
          value={config.voice_macro_trigger_word || ''}
          onInput={(e) => updateConfig('voice_macro_trigger_word', (e.target as HTMLInputElement).value)}
          placeholder="e.g. Computer (or leave blank)"
          style={{ ...inputBaseStyle, width: '100%' }}
        />
      </ConfigField>

      <ConfigField
        label="Audio Chime Alert"
        description="Play a subtle confirmation sound whenever a voice macro is recognized and fired."
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.sm }}>
          <Button
            variant="configAction"
            onClick={handleTestSound}
            disabled={isPlayingTestSound.value}
            style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
          >
            <IconVolume size={14} />
            <span>Test Sound</span>
          </Button>
          <Switch
            name="Audio Chime Alert"
            checked={config.voice_macro_sound_feedback}
            onChange={(checked) => updateConfig('voice_macro_sound_feedback', checked)}
          />
        </div>
      </ConfigField>

      <ConfigField
        label="Suppress Overlay"
        description="Keep the visual HUD overlay hidden during macro execution so it doesn't interrupt full-screen games."
      >
        <div style={{ display: 'flex', alignItems: 'center', height: '100%' }}>
          <Switch
            name="Suppress Overlay"
            checked={config.voice_macro_suppress_overlay}
            onChange={(checked) => updateConfig('voice_macro_suppress_overlay', checked)}
          />
        </div>
      </ConfigField>

      <ConfigField
        label="Voice Activation Threshold"
        description="Adjust the sound level required to trigger speech recognition. Higher values ignore breathing, fan hum, and background noise."
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
          <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, textAlign: 'left' }}>
            Threshold Level: {Math.round((config.voice_macro_activation_threshold || 0.035) * 1000)} / 100
          </div>
          <SliderField
            value={config.voice_macro_activation_threshold || 0.035}
            min={0.01}
            max={0.12}
            step={0.005}
            onChange={(val) => updateConfig('voice_macro_activation_threshold', val)}
            ariaLabel="Voice activation threshold"
            style={{ margin: `${tokens.spacing.sm} 0` }}
          />
        </div>
      </ConfigField>

      <ConfigField
        label="Configured Voice Commands"
        description="Record or build multi-step macro sequences with keypresses, holds, releases, and editable delays."
      >
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md, width: '100%' }}>
          <MacroSequenceBuilder
            newPhrase={newPhrase}
            newSteps={newSteps}
            editingMacroId={editingMacroId}
            isRecordingSequence={isRecordingSequence}
            manualKeyInput={manualKeyInput}
            editingDelayIndex={editingDelayIndex}
            editingDelayValue={editingDelayValue}
            onSave={handleAddCommand}
            onCancelEdit={handleCancelEdit}
            onToggleRecord={() => {
              if (isRecordingSequence.value) {
                isRecordingSequence.value = false;
              } else {
                isRecordingSequence.value = true;
                lastEventTime.value = 0;
                pressedKeysDownTime.value = {};
              }
            }}
            onRemoveStep={removeStep}
            onAddManualKey={addManualKey}
            onAddManualDelay={addManualDelay}
            onStartEditDelay={startEditDelay}
            onSaveEditDelay={saveEditDelay}
          />

          {/* List of Configured Macros */}
          {macros.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: tokens.spacing.xs }}>
              {macros.map((cmd) => {
                const steps = resolveMacroSteps(cmd);
                return (
                  <div
                    key={cmd.id}
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      padding: '10px 12px',
                      borderRadius: '8px',
                      background:
                        editingMacroId.value === cmd.id
                          ? 'rgba(88, 101, 242, 0.12)'
                          : 'rgba(255, 255, 255, 0.05)',
                      border:
                        editingMacroId.value === cmd.id
                          ? '1px solid #5865f2'
                          : '1px solid rgba(255, 255, 255, 0.08)',
                      gap: '8px',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span
                          style={{
                            fontSize: tokens.typography.sizeSm,
                            color: tokens.colors.textPrimary,
                            fontWeight: 600,
                          }}
                        >
                          "{cmd.phrase}"
                        </span>
                        <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
                          ({steps.length} {steps.length === 1 ? 'step' : 'steps'})
                        </span>
                        {editingMacroId.value === cmd.id && (
                          <span
                            style={{
                              fontSize: '10px',
                              color: '#9ba5ff',
                              fontWeight: 600,
                              padding: '1px 6px',
                              borderRadius: '4px',
                              background: 'rgba(88, 101, 242, 0.25)',
                            }}
                          >
                            Editing
                          </span>
                        )}
                      </div>

                      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <button
                          onClick={() => void handleTestMacro(cmd)}
                          disabled={isTestingExecution.value === cmd.id}
                          style={{
                            background: 'none',
                            border: 'none',
                            color: tokens.colors.textSecondary,
                            cursor: 'pointer',
                            padding: '4px',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            borderRadius: '4px',
                          }}
                          title="Test full macro sequence"
                        >
                          <IconPlayerPlay size={15} />
                        </button>
                        <button
                          onClick={() => handleStartEdit(cmd)}
                          style={{
                            background: 'none',
                            border: 'none',
                            color: tokens.colors.textSecondary,
                            cursor: 'pointer',
                            padding: '4px',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            borderRadius: '4px',
                          }}
                          title={`Edit "${cmd.phrase}" sequence`}
                        >
                          <IconPencil size={15} />
                        </button>
                        <button
                          onClick={() => handleDeleteCommand(cmd.id)}
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
                          title={`Remove "${cmd.phrase}"`}
                        >
                          <IconTrash size={15} />
                        </button>
                      </div>
                    </div>

                    {/* Sequence Steps Render */}
                    <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap' }}>
                      {steps.length > 0 ? (
                        steps.map((step, sIdx) => (
                          <div key={sIdx} style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
                            <MacroStepChip step={step} />
                            {sIdx < steps.length - 1 && (
                              <span style={{ color: tokens.colors.textMuted, fontSize: '9px' }}>➔</span>
                            )}
                          </div>
                        ))
                      ) : (
                        <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
                          No steps defined
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, padding: '8px 0' }}>
              No voice commands configured yet. Record or add a sequence above.
            </div>
          )}
        </div>
      </ConfigField>
    </>
  );
}
