import { useSignal } from '@preact/signals';
import { IconX } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { inputBaseStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

interface WordListEditorProps {
  words: string[];
  onChange: (words: string[]) => void;
  placeholder: string;
  /// Applied to a word before it is added, e.g. lower-casing.
  normalize?: (word: string) => string;
}

/// A text box that adds words to a list, with each word shown as a
/// removable chip. Duplicates are ignored.
export function WordListEditor({ words, onChange, placeholder, normalize = (word) => word }: WordListEditorProps) {
  const draft = useSignal('');

  const addWord = () => {
    const word = normalize(draft.value.trim());
    if (word && !words.includes(word)) {
      onChange([...words, word]);
    }
    draft.value = '';
  };

  const removeWord = (index: number) => {
    onChange(words.filter((_, position) => position !== index));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs, width: '100%' }}>
      <div style={{ display: 'flex', gap: tokens.spacing.xs, width: '100%' }}>
        <input
          type="text"
          value={draft.value}
          onInput={(event) => { draft.value = (event.target as HTMLInputElement).value; }}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              addWord();
            }
          }}
          placeholder={placeholder}
          style={{ ...inputBaseStyle, flex: 1 }}
        />
        <Button variant="configAction" onClick={addWord} disabled={!draft.value.trim()}>
          Add
        </Button>
      </div>
      {words.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginTop: tokens.spacing.xs }}>
          {words.map((word, index) => (
            <div
              key={word}
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
                onClick={() => removeWord(index)}
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
  );
}
