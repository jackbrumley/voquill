import { useSignal } from '@preact/signals';
import { IconX } from '@tabler/icons-preact';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { Button } from '../../components/Button.tsx';
import type { Config } from '../../types.ts';
import { inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';

interface FillerWordsSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
}

export function FillerWordsSection({ config, updateConfig }: FillerWordsSectionProps) {
  const fillerWordInput = useSignal('');

  const addFillerWord = () => {
    const trimmed = fillerWordInput.value.trim().toLowerCase();
    if (trimmed && !(config.custom_filler_words || []).includes(trimmed)) {
      updateConfig('custom_filler_words', [...(config.custom_filler_words || []), trimmed]);
    }
    fillerWordInput.value = '';
  };

  return (
    <>
      <ConfigField label="Remove Filler Words" description="Automatically remove filler words (uh, umm, hmm, etc.) from transcriptions using a fast regex pass. Works without post-processing.">
        <Switch
          name="Remove Filler Words"
          checked={config.filler_word_removal_enabled}
          onChange={(checked) => updateConfig('filler_word_removal_enabled', checked)}
        />
      </ConfigField>

      <ConfigField label="Custom Filler Words" description="Add words to also remove. Built-in words (uh, umm, hmm, etc.) are always removed. Add extra words like 'literally', 'actually', 'basically' if you tend to overuse them.">
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
          <div style={{ display: 'flex', gap: tokens.spacing.xs, width: '100%' }}>
            <input
              type="text"
              value={fillerWordInput.value}
              onInput={(e) => { fillerWordInput.value = (e.target as HTMLInputElement).value; }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  addFillerWord();
                }
              }}
              placeholder="e.g. literally, basically, actually"
              style={{ ...inputBaseStyle, flex: 1 }}
            />
            <Button
              variant="configAction"
              onClick={addFillerWord}
              disabled={!fillerWordInput.value.trim()}
            >
              Add
            </Button>
          </div>
          {(config.custom_filler_words || []).length > 0 && (
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginTop: tokens.spacing.xs }}>
              {(config.custom_filler_words || []).map((word, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    padding: '3px 8px',
                    borderRadius: '6px',
                    background: 'rgba(255,255,255,0.06)',
                    fontSize: tokens.typography.sizeXs,
                    color: tokens.colors.textPrimary,
                  }}
                >
                  <span>{word}</span>
                  <button
                    onClick={() => {
                      const updated = [...(config.custom_filler_words || [])];
                      updated.splice(i, 1);
                      updateConfig('custom_filler_words', updated);
                    }}
                    style={{
                      background: 'none',
                      border: 'none',
                      color: tokens.colors.textMuted,
                      cursor: 'pointer',
                      padding: '0',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      lineHeight: 1,
                    }}
                    title={`Remove "${word}"`}
                  >
                    <IconX size={12} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </ConfigField>
    </>
  );
}
