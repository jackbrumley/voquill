import { useSignal } from '@preact/signals';
import { useEffect } from 'preact/hooks';
import {
  IconVolume,
  IconVolumeOff,
  IconMicrophone,
  IconPlayerPlay,
  IconPencil,
  IconPlus,
  IconCopy,
  IconShare,
  IconDownload,
  IconUpload,
  IconAdjustmentsHorizontal,
  IconDotsVertical,
} from '@tabler/icons-preact';
import { invoke } from '@tauri-apps/api/core';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { Button } from '../../components/Button.tsx';
import { SliderField } from '../../components/SliderField.tsx';
import type { Config, MacroSoundMode, MacroStep, VoiceMacroCommand } from '../../types.ts';
import { inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import { resolveMacroSteps } from './voice_macro/keyUtils.ts';
import { MacroEditorModal } from './voice_macro/MacroEditorModal.tsx';
import { SpokenMacroTester } from './voice_macro/SpokenMacroTester.tsx';
import { MacroImportModal } from './voice_macro/MacroImportModal.tsx';
import { VoiceLabModal } from './voice_macro/VoiceLabModal.tsx';
import {
  cloneMacro,
  generateMacroId,
  serializeSingleMacro,
  serializeMacroBundle,
} from './voice_macro/macroSharing.ts';

interface VoiceMacrosSectionProps {
  config: Config;
  updateConfig: (
    key: string,
    value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]
  ) => void;
  showToast?: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

const actionButtonStyle = {
  background: 'none',
  border: 'none',
  color: tokens.colors.textSecondary,
  cursor: 'pointer',
  padding: '3px',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  borderRadius: '3px',
};

const dropdownItemStyle = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '7px 10px',
  borderRadius: '6px',
  background: 'transparent',
  border: 'none',
  color: tokens.colors.textPrimary,
  fontSize: '11.5px',
  fontWeight: 500,
  cursor: 'pointer',
  width: '100%',
  textAlign: 'left' as const,
  transition: tokens.transitions.fast,
};

export function VoiceMacrosSection({ config, updateConfig, showToast }: VoiceMacrosSectionProps) {
  const isEditorModalOpen = useSignal(false);
  const isImportModalOpen = useSignal(false);
  const isVoiceLabModalOpen = useSignal(false);
  const isMenuOpen = useSignal(false);
  const editingCommand = useSignal<VoiceMacroCommand | null>(null);
  const isPlayingTestSound = useSignal(false);
  const isTestingExecution = useSignal<string | null>(null);

  useEffect(() => {
    if (!isMenuOpen.value) return;
    const handleDocumentClick = () => {
      isMenuOpen.value = false;
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        isMenuOpen.value = false;
      }
    };
    window.addEventListener('click', handleDocumentClick);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('click', handleDocumentClick);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isMenuOpen.value]);

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

  const handleOpenCreateModal = () => {
    editingCommand.value = null;
    isEditorModalOpen.value = true;
  };

  const handleOpenEditModal = (cmd: VoiceMacroCommand) => {
    editingCommand.value = cmd;
    isEditorModalOpen.value = true;
  };

  const handleCloseModal = () => {
    isEditorModalOpen.value = false;
    editingCommand.value = null;
  };

  const handleSaveModal = (
    phrase: string,
    phrases: string[],
    steps: MacroStep[],
    soundMode?: MacroSoundMode,
    soundTtsText?: string | null,
    soundTtsVoice?: string | null,
    soundTtsSpeed?: number | null,
    soundTtsEffect?: string | null,
    soundTtsPitch?: number | null
  ) => {
    const currentMacros = config.voice_macros || [];

    if (editingCommand.value) {
      const editId = editingCommand.value.id;
      const updated = currentMacros.map((cmd) => {
        if (cmd.id === editId) {
          return {
            ...cmd,
            phrase,
            phrases: [...phrases],
            steps: [...steps],
            key_combination: null,
            hold_ms: null,
            delay_after_ms: null,
            sound_mode: soundMode || 'default',
            sound_tts_text: soundTtsText || null,
            sound_tts_voice: soundTtsVoice || null,
            sound_tts_speed: soundTtsSpeed || null,
            sound_tts_effect: soundTtsEffect || null,
            sound_tts_pitch: soundTtsPitch ?? null,
          };
        }
        return cmd;
      });
      updateConfig('voice_macros', updated);
      showToast?.(`Updated macro "${phrase}"`, 'success');
    } else {
      const newCommand: VoiceMacroCommand = {
        id: generateMacroId(),
        phrase,
        phrases: [...phrases],
        steps: [...steps],
        key_combination: null,
        hold_ms: null,
        delay_after_ms: null,
        sound_mode: soundMode || 'default',
        sound_tts_text: soundTtsText || null,
        sound_tts_voice: soundTtsVoice || null,
        sound_tts_speed: soundTtsSpeed || null,
        sound_tts_effect: soundTtsEffect || null,
        sound_tts_pitch: soundTtsPitch ?? null,
      };
      updateConfig('voice_macros', [...currentMacros, newCommand]);
      showToast?.(`Added macro command "${phrase}"`, 'success');
    }

    handleCloseModal();
  };

  const handleSaveAsCopy = (
    phrase: string,
    phrases: string[],
    steps: MacroStep[],
    soundMode?: MacroSoundMode,
    soundTtsText?: string | null,
    soundTtsVoice?: string | null,
    soundTtsSpeed?: number | null,
    soundTtsEffect?: string | null,
    soundTtsPitch?: number | null
  ) => {
    const currentMacros = config.voice_macros || [];
    const newCommand: VoiceMacroCommand = {
      id: generateMacroId(),
      phrase,
      phrases: [...phrases],
      steps: [...steps],
      key_combination: null,
      hold_ms: null,
      delay_after_ms: null,
      sound_mode: soundMode || 'default',
      sound_tts_text: soundTtsText || null,
      sound_tts_voice: soundTtsVoice || null,
      sound_tts_speed: soundTtsSpeed || null,
      sound_tts_effect: soundTtsEffect || null,
      sound_tts_pitch: soundTtsPitch ?? null,
    };
    updateConfig('voice_macros', [...currentMacros, newCommand]);
    showToast?.(`Created copy "${phrase}"`, 'success');
    handleCloseModal();
  };

  const handleDuplicateMacro = (cmd: VoiceMacroCommand) => {
    const currentMacros = config.voice_macros || [];
    const cloned = cloneMacro(cmd);
    const existingPhrases = new Set(currentMacros.map((m) => m.phrase.trim().toLowerCase()));
    let phrase = cloned.phrase;
    let counter = 2;
    while (existingPhrases.has(phrase)) {
      phrase = `copy of ${cmd.phrase} (${counter})`;
      counter++;
    }
    cloned.phrase = phrase;
    updateConfig('voice_macros', [...currentMacros, cloned]);
    showToast?.(`Duplicated macro "${cmd.phrase}"`, 'success');
  };

  const handleShareMacro = async (cmd: VoiceMacroCommand) => {
    const json = serializeSingleMacro(cmd);
    try {
      await navigator.clipboard.writeText(json);
      showToast?.(`Copied macro "${cmd.phrase}" JSON to clipboard`, 'success');
    } catch {
      showToast?.('Failed to copy to clipboard', 'error');
    }
  };

  const handleExportAllMacros = async () => {
    const currentMacros = config.voice_macros || [];
    if (currentMacros.length === 0) {
      showToast?.('No macros to export', 'info');
      return;
    }
    const json = serializeMacroBundle(currentMacros);
    try {
      await navigator.clipboard.writeText(json);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `voquill-macros-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      showToast?.(`Exported ${currentMacros.length} macros to JSON file & clipboard`, 'success');
    } catch {
      showToast?.('Failed to export macros', 'error');
    }
  };

  const handleImportMacros = (newMacros: VoiceMacroCommand[]) => {
    const currentMacros = config.voice_macros || [];
    updateConfig('voice_macros', [...currentMacros, ...newMacros]);
    isImportModalOpen.value = false;
    showToast?.(
      `Successfully imported ${newMacros.length} ${newMacros.length === 1 ? 'macro' : 'macros'}`,
      'success'
    );
  };

  const handleDeleteCommand = (id: string, phrase: string) => {
    const currentMacros = config.voice_macros || [];
    const updated = currentMacros.filter((m) => m.id !== id);
    updateConfig('voice_macros', updated);
    showToast?.(`Removed macro "${phrase}"`, 'info');
    handleCloseModal();
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
          onInput={(e) =>
            updateConfig('voice_macro_trigger_word', (e.target as HTMLInputElement).value)
          }
          placeholder="e.g. Computer (or leave blank)"
          style={{ ...inputBaseStyle, width: '100%' }}
        />
      </ConfigField>

      <ConfigField
        label="Audio Chime Alert"
        description="Play a subtle confirmation sound whenever a voice macro is recognized and fired."
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            width: '100%',
            gap: tokens.spacing.sm,
          }}
        >
          <Button
            variant="configAction"
            onClick={handleTestSound}
            disabled={isPlayingTestSound.value}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              whiteSpace: 'nowrap',
              flexShrink: 0,
              padding: '4px 10px',
              fontSize: '11px',
            }}
          >
            <IconVolume size={14} />
            <span style={{ whiteSpace: 'nowrap' }}>Test Sound</span>
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
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: tokens.spacing.xs,
            width: '100%',
          }}
        >
          <div
            style={{
              fontSize: tokens.typography.sizeXs,
              color: tokens.colors.textMuted,
              textAlign: 'left',
            }}
          >
            Threshold Level: {Math.round((config.voice_macro_activation_threshold || 0.035) * 1000)}{' '}
            / 100
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
        description="Record and customize multi-step key sequences, holds, releases, and precise delays."
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: tokens.spacing.md,
            width: '100%',
          }}
        >
          <SpokenMacroTester showToast={showToast} />

          {/* Action header with Voice Studio, Create Macro, and Three-Dot Utility Menu */}
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '8px',
              position: 'relative',
            }}
          >
            <span
              style={{
                fontSize: '13px',
                fontWeight: 600,
                color: tokens.colors.textSecondary,
              }}
            >
              Custom Macros ({macros.length})
            </span>

            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', position: 'relative' }}>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  isVoiceLabModalOpen.value = true;
                }}
                title="Open Voice Studio to design and tune custom AI voice presets"
              >
                <IconAdjustmentsHorizontal size={13} />
                <span>Voice Studio</span>
              </Button>

              <Button
                variant="configAction"
                size="sm"
                onClick={handleOpenCreateModal}
                title="Create a new voice macro"
                style={{ padding: '5px 10px' }}
              >
                <IconPlus size={14} />
                <span>New</span>
              </Button>

              <div style={{ position: 'relative' }}>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={(e) => {
                    e.stopPropagation();
                    isMenuOpen.value = !isMenuOpen.value;
                  }}
                  title="More actions (Import / Export)"
                  style={{ padding: '5px 8px', minWidth: '28px' }}
                >
                  <IconDotsVertical size={14} />
                </Button>

                {isMenuOpen.value && (
                  <div
                    onClick={(e) => e.stopPropagation()}
                    style={{
                      position: 'absolute',
                      top: 'calc(100% + 4px)',
                      right: 0,
                      zIndex: 100,
                      minWidth: '160px',
                      background: 'rgba(26, 29, 38, 0.95)',
                      backdropFilter: 'blur(16px)',
                      WebkitBackdropFilter: 'blur(16px)',
                      border: '1px solid rgba(255, 255, 255, 0.12)',
                      borderRadius: '10px',
                      padding: '4px',
                      boxShadow: tokens.shadows.lg,
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '2px',
                    }}
                  >
                    <button
                      type="button"
                      onClick={() => {
                        isMenuOpen.value = false;
                        isImportModalOpen.value = true;
                      }}
                      style={dropdownItemStyle}
                      onMouseEnter={(e) => {
                        (e.currentTarget as HTMLButtonElement).style.background = 'rgba(255, 255, 255, 0.08)';
                      }}
                      onMouseLeave={(e) => {
                        (e.currentTarget as HTMLButtonElement).style.background = 'transparent';
                      }}
                    >
                      <IconUpload size={13} />
                      <span>Import Macros...</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => {
                        isMenuOpen.value = false;
                        handleExportAllMacros();
                      }}
                      disabled={macros.length === 0}
                      style={{
                        ...dropdownItemStyle,
                        opacity: macros.length === 0 ? 0.4 : 1,
                        cursor: macros.length === 0 ? 'not-allowed' : 'pointer',
                      }}
                      onMouseEnter={(e) => {
                        if (macros.length > 0) {
                          (e.currentTarget as HTMLButtonElement).style.background = 'rgba(255, 255, 255, 0.08)';
                        }
                      }}
                      onMouseLeave={(e) => {
                        (e.currentTarget as HTMLButtonElement).style.background = 'transparent';
                      }}
                    >
                      <IconDownload size={13} />
                      <span>Export All ({macros.length})</span>
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* List of Configured Macros */}
          {macros.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              {macros.map((cmd) => {
                const steps = resolveMacroSteps(cmd);
                return (
                  <div
                    key={cmd.id}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '8px 10px',
                      borderRadius: '6px',
                      background: 'rgba(255, 255, 255, 0.04)',
                      border: '1px solid rgba(255, 255, 255, 0.08)',
                      gap: '6px',
                    }}
                  >
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '6px',
                        flexWrap: 'wrap',
                        minWidth: 0,
                        flex: 1,
                      }}
                    >
                      <span
                        style={{
                          fontSize: '13px',
                          color: tokens.colors.textPrimary,
                          fontWeight: 600,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        "{cmd.phrase}"
                      </span>
                      {config.voice_macro_trigger_word && (
                        <span
                          style={{
                            fontSize: '9.5px',
                            color: '#93c5fd',
                            background: 'rgba(59, 130, 246, 0.18)',
                            border: '1px solid rgba(59, 130, 246, 0.35)',
                            padding: '1px 5px',
                            borderRadius: '4px',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          Prefix: {config.voice_macro_trigger_word}
                        </span>
                      )}
                      {cmd.phrases && cmd.phrases.length > 0 && (
                        <span
                          style={{
                            fontSize: '9.5px',
                            color: '#cbd5e1',
                            background: 'rgba(255, 255, 255, 0.06)',
                            border: '1px solid rgba(255, 255, 255, 0.12)',
                            padding: '1px 5px',
                            borderRadius: '4px',
                            whiteSpace: 'nowrap',
                          }}
                          title={`Aliases: ${cmd.phrases.map((p) => `"${p}"`).join(', ')}`}
                        >
                          +{cmd.phrases.length} {cmd.phrases.length === 1 ? 'alias' : 'aliases'}
                        </span>
                      )}
                      <span
                        style={{
                          fontSize: '11px',
                          color: tokens.colors.textMuted,
                          whiteSpace: 'nowrap',
                        }}
                      >
                        • {steps.length} {steps.length === 1 ? 'step' : 'steps'}
                      </span>

                      {cmd.sound_mode === 'tts' && cmd.sound_tts_text && (
                        <span
                          style={{
                            fontSize: '9.5px',
                            color: '#f472b6',
                            background: 'rgba(236, 72, 153, 0.15)',
                            border: '1px solid rgba(236, 72, 153, 0.3)',
                            padding: '1px 5px',
                            borderRadius: '4px',
                            whiteSpace: 'nowrap',
                            maxWidth: '140px',
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '3px',
                          }}
                          title={`Voice feedback: "${cmd.sound_tts_text}"`}
                        >
                          <IconVolume size={11} />
                          <span>"{cmd.sound_tts_text}"</span>
                        </span>
                      )}
                      {cmd.sound_mode === 'custom_file' && (
                        <span
                          style={{
                            fontSize: '9.5px',
                            color: '#34d399',
                            background: 'rgba(16, 185, 129, 0.15)',
                            border: '1px solid rgba(16, 185, 129, 0.3)',
                            padding: '1px 5px',
                            borderRadius: '4px',
                            whiteSpace: 'nowrap',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '3px',
                          }}
                        >
                          <IconUpload size={11} />
                          <span>Audio</span>
                        </span>
                      )}
                      {cmd.sound_mode === 'mic_recording' && (
                        <span
                          style={{
                            fontSize: '9.5px',
                            color: '#f87171',
                            background: 'rgba(239, 68, 68, 0.15)',
                            border: '1px solid rgba(239, 68, 68, 0.3)',
                            padding: '1px 5px',
                            borderRadius: '4px',
                            whiteSpace: 'nowrap',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '3px',
                          }}
                        >
                          <IconMicrophone size={11} />
                          <span>Voice Clip</span>
                        </span>
                      )}
                      {cmd.sound_mode === 'none' && (
                        <span
                          style={{
                            fontSize: '9.5px',
                            color: tokens.colors.textMuted,
                            background: 'rgba(255, 255, 255, 0.05)',
                            border: '1px solid rgba(255, 255, 255, 0.1)',
                            padding: '1px 5px',
                            borderRadius: '4px',
                            whiteSpace: 'nowrap',
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '3px',
                          }}
                        >
                          <IconVolumeOff size={11} />
                          <span>Mute</span>
                        </span>
                      )}
                    </div>

                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: '3px',
                        flexShrink: 0,
                      }}
                    >
                      <button
                        onClick={() => void handleTestMacro(cmd)}
                        disabled={isTestingExecution.value === cmd.id}
                        style={actionButtonStyle}
                        title="Test macro sequence"
                      >
                        <IconPlayerPlay size={14} />
                      </button>
                      <button
                        onClick={() => handleDuplicateMacro(cmd)}
                        style={actionButtonStyle}
                        title={`Duplicate "${cmd.phrase}"`}
                      >
                        <IconCopy size={14} />
                      </button>
                      <button
                        onClick={() => handleShareMacro(cmd)}
                        style={actionButtonStyle}
                        title={`Copy "${cmd.phrase}" JSON to clipboard`}
                      >
                        <IconShare size={14} />
                      </button>
                      <button
                        onClick={() => handleOpenEditModal(cmd)}
                        style={actionButtonStyle}
                        title={`Edit "${cmd.phrase}" sequence`}
                      >
                        <IconPencil size={14} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                padding: '24px 16px',
                borderRadius: '8px',
                background: 'rgba(255, 255, 255, 0.02)',
                border: '1px dashed rgba(255, 255, 255, 0.08)',
                gap: '8px',
              }}
            >
              <span style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textMuted }}>
                No voice macros configured yet.
              </span>
              <Button variant="configAction" onClick={handleOpenCreateModal}>
                + Create Your First Macro
              </Button>
            </div>
          )}
        </div>
      </ConfigField>

      {/* Dedicated Macro Editor Modal */}
      {isEditorModalOpen.value && (
        <MacroEditorModal
          initialCommand={editingCommand.value}
          onSave={handleSaveModal}
          onSaveAsCopy={handleSaveAsCopy}
          onDelete={
            editingCommand.value
              ? () => handleDeleteCommand(editingCommand.value!.id, editingCommand.value!.phrase)
              : undefined
          }
          onClose={handleCloseModal}
        />
      )}

      {/* Dedicated Macro Import Modal */}
      {isImportModalOpen.value && (
        <MacroImportModal
          existingMacros={macros}
          onImport={handleImportMacros}
          onClose={() => {
            isImportModalOpen.value = false;
          }}
        />
      )}

      {/* Voice Studio Modal */}
      {isVoiceLabModalOpen.value && (
        <VoiceLabModal
          onClose={() => {
            isVoiceLabModalOpen.value = false;
          }}
        />
      )}
    </>
  );
}
