import { useSignal } from '@preact/signals';
import { IconVolume } from '@tabler/icons-preact';
import { invoke } from '@tauri-apps/api/core';
import { AdvancedSettings } from '../../components/AdvancedSettings.tsx';
import { Button } from '../../components/Button.tsx';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import type { Config } from '../../types.ts';
import { inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import { MacroEditorModal } from './voice_macro/MacroEditorModal.tsx';
import { MacroImportModal } from './voice_macro/MacroImportModal.tsx';
import { MacroLibraryHeader } from './voice_macro/MacroLibraryHeader.tsx';
import { MacroListRow } from './voice_macro/MacroListRow.tsx';
import { SpokenMacroTester } from './voice_macro/SpokenMacroTester.tsx';
import { VoiceLabModal } from './voice_macro/VoiceLabModal.tsx';
import { useMacroLibrary } from './voice_macro/useMacroLibrary.ts';

interface VoiceMacrosSectionProps {
  config: Config;
  updateConfig: (
    key: string,
    value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]
  ) => void;
  showToast?: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

export function VoiceMacrosSection({ config, updateConfig, showToast }: VoiceMacrosSectionProps) {
  const library = useMacroLibrary({ config, updateConfig, showToast });
  const isVoiceStudioOpen = useSignal(false);
  const isTesterOpen = useSignal(false);
  const isPlayingTestSound = useSignal(false);

  const playTestChime = async () => {
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

  const editingMacro = library.editingMacro;

  return (
    <>
      <ConfigField
        label="Voice Macros"
        description="Say a phrase to run a shortcut or action, without pressing anything."
      >
        <div style={{ display: 'flex', alignItems: 'center', height: '100%' }}>
          <Switch
            name="Voice Macros"
            checked={config.voice_macros_enabled}
            onChange={(checked) => updateConfig('voice_macros_enabled', checked)}
          />
        </div>
      </ConfigField>

      {config.voice_macros_enabled && (
        <>
          <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md, width: '100%' }}>
            <MacroLibraryHeader
              macroCount={library.macros.length}
              isTesterOpen={isTesterOpen.value}
              onToggleTester={() => { isTesterOpen.value = !isTesterOpen.value; }}
              onOpenVoiceStudio={() => { isVoiceStudioOpen.value = true; }}
              onCreate={() => library.openEditor(null)}
              onImport={library.openImport}
              onExportAll={() => void library.exportAllMacros()}
            />

            {isTesterOpen.value && <SpokenMacroTester showToast={showToast} />}
            {library.macros.length > 0 ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                {library.macros.map((macro) => (
                  <MacroListRow
                    key={macro.id}
                    macro={macro}
                    triggerWord={config.voice_macro_trigger_word}
                    isTesting={library.testingMacroId === macro.id}
                    onTest={() => void library.testMacro(macro)}
                    onDuplicate={() => void library.duplicateMacro(macro)}
                    onShare={() => void library.shareMacro(macro)}
                    onEdit={() => library.openEditor(macro)}
                  />
                ))}
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
                <Button variant="configAction" onClick={() => library.openEditor(null)}>
                  + Create Your First Macro
                </Button>
              </div>
            )}
          </div>

          <AdvancedSettings>
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
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%', gap: tokens.spacing.sm }}>
                <Button
                  variant="configAction"
                  onClick={playTestChime}
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
          </AdvancedSettings>
        </>
      )}

      {library.isEditorOpen && (
        <MacroEditorModal
          initialCommand={editingMacro}
          onSave={library.saveMacro}
          onSaveAsCopy={library.saveMacroAsCopy}
          onDelete={editingMacro ? () => library.deleteMacro(editingMacro) : undefined}
          onClose={library.closeEditor}
        />
      )}

      {library.isImportOpen && (
        <MacroImportModal existingMacros={library.macros} onImport={library.importMacros} onClose={library.closeImport} />
      )}

      {isVoiceStudioOpen.value && <VoiceLabModal onClose={() => { isVoiceStudioOpen.value = false; }} />}
    </>
  );
}
