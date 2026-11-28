import { useSignal } from '@preact/signals';
import {
  IconCheck,
  IconTrash,
  IconCopy,
} from '@tabler/icons-preact';
import { confirm } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { Modal } from '../../../components/Modal.tsx';
import { Button } from '../../../components/Button.tsx';
import type { MacroSoundMode, VoiceMacroCommand } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { MacroTriggerStep } from './MacroTriggerStep.tsx';
import { MacroSequenceStep } from './MacroSequenceStep.tsx';
import { MacroSoundStep } from './MacroSoundStep.tsx';
import { useMacroSequence } from './useMacroSequence.ts';
import { generateMacroId } from './macroSharing.ts';

interface MacroEditorModalProps {
  initialCommand: VoiceMacroCommand | null;
  /// Receives the finished macro; its id is the edited macro's id.
  onSave: (command: VoiceMacroCommand) => void;
  /// Receives a copy with a new id and, when unchanged, a "copy of" phrase.
  onSaveAsCopy?: (command: VoiceMacroCommand) => void;
  onDelete?: () => void;
  onClose: () => void;
}

type EditorTab = 'trigger' | 'sequence' | 'sound';

export function MacroEditorModal({
  initialCommand,
  onSave,
  onSaveAsCopy,
  onDelete,
  onClose,
}: MacroEditorModalProps) {
  const activeTab = useSignal<EditorTab>('trigger');

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
  const soundMode = useSignal<MacroSoundMode>(initialCommand?.sound_mode || 'default');
  const soundTtsText = useSignal<string>(initialCommand?.sound_tts_text || '');
  const soundTtsVoice = useSignal<string>(initialCommand?.sound_tts_voice || '');
  const soundTtsSpeed = useSignal<number>(initialCommand?.sound_tts_speed || 1.0);
  const soundTtsEffect = useSignal<string>(initialCommand?.sound_tts_effect || 'custom');
  const soundTtsPitch = useSignal<number>(initialCommand?.sound_tts_pitch ?? 0);
  const macroId = useSignal<string>(initialCommand?.id || generateMacroId());

  const sequence = useMacroSequence(initialCommand?.steps || []);

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

  /// Every phrase the user entered, including one still in the input box;
  /// null when the macro is not complete enough to save.
  const collectPhrases = (): string[] | null => {
    const all = [...phrases.value];
    const pending = phraseInput.value.replace(/,/g, '').trim().toLowerCase();
    if (pending && !all.includes(pending)) {
      all.push(pending);
    }
    return all.length === 0 || sequence.steps.value.length === 0 ? null : all;
  };

  const buildCommand = (id: string, phrase: string, aliases: string[], enabled: boolean): VoiceMacroCommand => ({
    id,
    enabled,
    phrase,
    phrases: aliases,
    steps: [...sequence.steps.value],
    key_combination: null,
    hold_ms: null,
    delay_after_ms: null,
    sound_mode: soundMode.value,
    sound_tts_text: soundTtsText.value.trim() || null,
    sound_tts_voice: soundTtsVoice.value || null,
    sound_tts_speed: soundTtsSpeed.value || 1.0,
    sound_tts_effect: soundTtsEffect.value || null,
    sound_tts_pitch: soundTtsPitch.value,
  });

  /// Renders the spoken feedback to disk ahead of time so the macro plays it
  /// instantly when it fires.
  const prerenderSpokenFeedback = async (macroId: string) => {
    if (soundMode.value !== 'tts' || !soundTtsText.value.trim() || !soundTtsVoice.value) {
      return;
    }
    try {
      await invoke('save_macro_tts_audio', {
        macroId,
        text: soundTtsText.value.trim(),
        voiceId: soundTtsVoice.value,
        speed: soundTtsSpeed.value || 1.0,
        effect: soundTtsEffect.value || 'custom',
        pitch: soundTtsPitch.value ?? 0.0,
      });
    } catch (e) {
      console.warn('Failed to pre-render TTS audio:', e);
    }
  };

  const handleSave = async () => {
    const all = collectPhrases();
    if (!all) return;
    await prerenderSpokenFeedback(macroId.value);
    onSave(buildCommand(macroId.value, all[0], all.slice(1), initialCommand?.enabled ?? true));
  };

  const handleSaveAsCopy = async () => {
    const all = collectPhrases();
    if (!all || !onSaveAsCopy) return;
    const primaryPhrase = initialCommand && all[0] === initialCommand.phrase ? `copy of ${all[0]}` : all[0];
    const newId = generateMacroId();
    await prerenderSpokenFeedback(newId);
    onSaveAsCopy(buildCommand(newId, primaryPhrase, all.slice(1), true));
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

  const isSaveDisabled =
    totalPhraseCount === 0 || sequence.steps.value.length === 0 || sequence.isRecording.value;

  const tabs: { id: EditorTab; label: string }[] = [
    {
      id: 'trigger',
      label: phrases.value.length > 0 ? `Trigger (${phrases.value.length})` : 'Trigger',
    },
    {
      id: 'sequence',
      label: sequence.steps.value.length > 0 ? `Sequence (${sequence.steps.value.length})` : 'Sequence',
    },
    {
      id: 'sound',
      label: soundMode.value === 'tts' ? 'Sound (Voice)' : soundMode.value === 'none' ? 'Sound (Mute)' : 'Sound',
    },
  ];

  return (
    <Modal
      onClose={onClose}
      showCloseButton={false}
      footerAlign="space-between"
      topBar={
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, 1fr)',
            gap: '4px',
            background: 'rgba(0, 0, 0, 0.25)',
            padding: '3px',
            borderRadius: '999px',
            border: '1px solid rgba(255, 255, 255, 0.08)',
          }}
        >
          {tabs.map((tab) => {
            const isActive = activeTab.value === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => {
                  activeTab.value = tab.id;
                }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: '6px 8px',
                  borderRadius: '999px',
                  background: isActive
                    ? 'linear-gradient(135deg, rgba(88, 101, 242, 0.35) 0%, rgba(129, 140, 248, 0.2) 100%)'
                    : 'transparent',
                  border: isActive
                    ? '1px solid rgba(99, 102, 241, 0.55)'
                    : '1px solid transparent',
                  color: isActive ? '#ffffff' : tokens.colors.textSecondary,
                  fontSize: '11.5px',
                  fontWeight: isActive ? 600 : 500,
                  cursor: 'pointer',
                  transition: tokens.transitions.fast,
                  whiteSpace: 'nowrap',
                }}
              >
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>
      }
      footer={
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            width: '100%',
            gap: '8px',
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
                  padding: '5px 10px',
                  fontSize: '11.5px',
                }}
              >
                <IconTrash size={13} />
                <span>Delete</span>
              </Button>
            )}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Button
              variant="ghost"
              onClick={onClose}
              style={{ padding: '5px 10px', fontSize: '11.5px' }}
            >
              Cancel
            </Button>
            {initialCommand && onSaveAsCopy && (
              <Button
                variant="ghost"
                onClick={handleSaveAsCopy}
                disabled={isSaveDisabled}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '4px',
                  padding: '5px 10px',
                  fontSize: '11.5px',
                }}
                title="Save these steps as a new duplicate macro"
              >
                <IconCopy size={13} />
                <span>Save as Copy</span>
              </Button>
            )}
            <Button
              variant="configAction"
              onClick={handleSave}
              disabled={isSaveDisabled}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                padding: '5px 12px',
                fontSize: '12px',
                fontWeight: 600,
              }}
            >
              <IconCheck size={14} />
              <span>{initialCommand ? 'Save' : 'Create'}</span>
            </Button>
          </div>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', width: '100%' }}>
        {/* Tab 1: Trigger Step */}
        {activeTab.value === 'trigger' && (
          <MacroTriggerStep
            phrases={phrases.value}
            phraseInput={phraseInput.value}
            onPhraseInputChange={(val) => {
              phraseInput.value = val;
            }}
            onAddPhrase={addPhrase}
            onRemovePhrase={removePhrase}
            autoFocus={!initialCommand}
          />
        )}

        {/* Tab 2: Sequence Step */}
        {activeTab.value === 'sequence' && <MacroSequenceStep sequence={sequence} />}

        {/* Tab 3: Sound Feedback Step */}
        {activeTab.value === 'sound' && (
          <MacroSoundStep
            macroId={macroId.value}
            soundMode={soundMode.value}
            onSoundModeChange={(mode) => {
              soundMode.value = mode;
            }}
            ttsText={soundTtsText.value}
            onTtsTextChange={(val) => {
              soundTtsText.value = val;
            }}
            ttsVoice={soundTtsVoice.value}
            onTtsVoiceChange={(val) => {
              soundTtsVoice.value = val;
            }}
            ttsSpeed={soundTtsSpeed.value}
            onTtsSpeedChange={(val) => {
              soundTtsSpeed.value = val;
            }}
            ttsEffect={soundTtsEffect.value}
            onTtsEffectChange={(val) => {
              soundTtsEffect.value = val;
            }}
            ttsPitch={soundTtsPitch.value}
            onTtsPitchChange={(val) => {
              soundTtsPitch.value = val;
            }}
          />
        )}
      </div>
    </Modal>
  );
}
