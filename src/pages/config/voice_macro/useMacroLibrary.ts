import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { Config, VoiceMacroCommand } from '../../../types.ts';
import { resolveMacroSteps } from './keyUtils.ts';
import { cloneMacro, serializeMacroBundle, serializeSingleMacro } from './macroSharing.ts';

type ShowToast = (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;

interface UseMacroLibraryOptions {
  config: Config;
  updateConfig: (key: string, value: VoiceMacroCommand[]) => void;
  showToast?: ShowToast;
}

/// Owns the user's voice macro list: creating, editing, copying, sharing,
/// importing, exporting, deleting and test-running macros, plus the editor
/// and import dialogs those actions open. Every change to the list goes
/// through `saveMacros`, the one place that writes `config.voice_macros`.
export function useMacroLibrary({ config, updateConfig, showToast }: UseMacroLibraryOptions) {
  const isEditorOpen = useSignal(false);
  const editingMacro = useSignal<VoiceMacroCommand | null>(null);
  const isImportOpen = useSignal(false);
  const testingMacroId = useSignal<string | null>(null);

  const macros = config.voice_macros || [];
  const saveMacros = (next: VoiceMacroCommand[]) => updateConfig('voice_macros', next);

  const openEditor = (macro: VoiceMacroCommand | null) => {
    editingMacro.value = macro;
    isEditorOpen.value = true;
  };

  const closeEditor = () => {
    isEditorOpen.value = false;
    editingMacro.value = null;
  };

  /// Replaces the macro with the same id, or adds it when it is new.
  const saveMacro = (macro: VoiceMacroCommand) => {
    const exists = macros.some((existing) => existing.id === macro.id);
    saveMacros(exists ? macros.map((existing) => (existing.id === macro.id ? macro : existing)) : [...macros, macro]);
    showToast?.(exists ? `Updated macro "${macro.phrase}"` : `Added macro command "${macro.phrase}"`, 'success');
    closeEditor();
  };

  /// Switches a macro on or off without deleting it; a switched-off macro
  /// keeps its phrase but never runs.
  const setMacroEnabled = (macro: VoiceMacroCommand, enabled: boolean) => {
    saveMacros(macros.map((existing) => (existing.id === macro.id ? { ...existing, enabled } : existing)));
    showToast?.(enabled ? `Switched on "${macro.phrase}"` : `Switched off "${macro.phrase}"`, 'info');
  };

  const saveMacroAsCopy = (macro: VoiceMacroCommand) => {
    saveMacros([...macros, macro]);
    showToast?.(`Created copy "${macro.phrase}"`, 'success');
    closeEditor();
  };

  const duplicateMacro = async (macro: VoiceMacroCommand) => {
    const cloned = cloneMacro(macro);
    const existingPhrases = new Set(macros.map((m) => m.phrase.trim().toLowerCase()));
    let phrase = cloned.phrase;
    let counter = 2;
    while (existingPhrases.has(phrase)) {
      phrase = `copy of ${macro.phrase} (${counter})`;
      counter++;
    }
    cloned.phrase = phrase;

    if (cloned.sound_mode && cloned.sound_mode !== 'default' && cloned.sound_mode !== 'none') {
      try {
        await invoke('clone_macro_sound', { sourceMacroId: macro.id, targetMacroId: cloned.id });
      } catch (e) {
        console.warn('Failed to clone macro sound on disk:', e);
      }
    }

    saveMacros([...macros, cloned]);
    showToast?.(`Duplicated macro "${macro.phrase}"`, 'success');
  };

  const deleteMacro = (macro: VoiceMacroCommand) => {
    saveMacros(macros.filter((m) => m.id !== macro.id));
    invoke('delete_macro_sound', { macroId: macro.id }).catch((e) => {
      console.warn('Failed to delete macro sound on disk:', e);
    });
    showToast?.(`Removed macro "${macro.phrase}"`, 'info');
    closeEditor();
  };

  const shareMacro = async (macro: VoiceMacroCommand) => {
    try {
      await navigator.clipboard.writeText(serializeSingleMacro(macro));
      showToast?.(`Copied macro "${macro.phrase}" JSON to clipboard`, 'success');
    } catch {
      showToast?.('Failed to copy to clipboard', 'error');
    }
  };

  const exportAllMacros = async () => {
    if (macros.length === 0) {
      showToast?.('No macros to export', 'info');
      return;
    }
    const json = serializeMacroBundle(macros);
    try {
      await navigator.clipboard.writeText(json);
      const url = URL.createObjectURL(new Blob([json], { type: 'application/json' }));
      const link = document.createElement('a');
      link.href = url;
      link.download = `voquill-macros-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
      showToast?.(`Exported ${macros.length} macros to JSON file & clipboard`, 'success');
    } catch {
      showToast?.('Failed to export macros', 'error');
    }
  };

  const importMacros = (imported: VoiceMacroCommand[]) => {
    saveMacros([...macros, ...imported]);
    isImportOpen.value = false;
    showToast?.(`Successfully imported ${imported.length} ${imported.length === 1 ? 'macro' : 'macros'}`, 'success');
  };

  /// Plays the macro's feedback sound: its saved voice line, audio file or
  /// clip, or the default chime.
  const playMacroSound = async (macro: VoiceMacroCommand) => {
    try {
      await invoke('play_macro_sound_preview', { macroId: macro.id });
    } catch (e) {
      showToast?.(`Couldn't play the sound: ${e}`, 'error');
    }
  };

  const testMacro = async (macro: VoiceMacroCommand) => {
    testingMacroId.value = macro.id;
    const steps = resolveMacroSteps(macro);
    try {
      await invoke('test_voice_macro_execution', { steps });
      showToast?.(`Executed macro "${macro.phrase}" (${steps.length} steps)`, 'success');
    } catch (e) {
      showToast?.(`Failed to execute macro: ${e}`, 'error');
    } finally {
      setTimeout(() => {
        testingMacroId.value = null;
      }, 500);
    }
  };

  return {
    macros,
    isEditorOpen: isEditorOpen.value,
    editingMacro: editingMacro.value,
    isImportOpen: isImportOpen.value,
    testingMacroId: testingMacroId.value,
    openEditor,
    closeEditor,
    openImport: () => { isImportOpen.value = true; },
    closeImport: () => { isImportOpen.value = false; },
    saveMacro,
    saveMacroAsCopy,
    setMacroEnabled,
    duplicateMacro,
    deleteMacro,
    shareMacro,
    exportAllMacros,
    importMacros,
    testMacro,
    playMacroSound,
  };
}

export type MacroLibrary = ReturnType<typeof useMacroLibrary>;
