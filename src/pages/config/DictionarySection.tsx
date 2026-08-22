import { useSignal } from '@preact/signals';
import { IconX } from '@tabler/icons-preact';
import { ConfigField } from '../../components/ConfigField.tsx';
import { Button } from '../../components/Button.tsx';
import type { Config } from '../../types.ts';
import { inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';

interface DictionarySectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
}

export function DictionarySection({ config, updateConfig }: DictionarySectionProps) {
  const dictionaryInput = useSignal('');

  const addWord = () => {
    const trimmed = dictionaryInput.value.trim();
    if (trimmed && !(config.dictionary || []).includes(trimmed)) {
      updateConfig('dictionary', [...(config.dictionary || []), trimmed]);
    }
    dictionaryInput.value = '';
  };

  return (
    <ConfigField label="Custom Words" description="Add names, jargon, or terms Whisper often gets wrong. Helps improve accuracy.">
      <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
        <div style={{ display: 'flex', gap: tokens.spacing.xs, width: '100%' }}>
          <input
            type="text"
            value={dictionaryInput.value}
            onInput={(e) => { dictionaryInput.value = (e.target as HTMLInputElement).value; }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                addWord();
              }
            }}
            placeholder="e.g. Anthropic, Rust, Voquill"
            style={{ ...inputBaseStyle, flex: 1 }}
          />
          <Button
            variant="configAction"
            onClick={addWord}
            disabled={!dictionaryInput.value.trim()}
          >
            Add
          </Button>
        </div>
        {(config.dictionary || []).length > 0 && (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginTop: tokens.spacing.xs }}>
            {(config.dictionary || []).map((word, i) => (
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
                    const updated = [...(config.dictionary || [])];
                    updated.splice(i, 1);
                    updateConfig('dictionary', updated);
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
  );
}
