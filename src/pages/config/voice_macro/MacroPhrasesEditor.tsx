import { IconPlus, IconX } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { tokens } from '../../../design-tokens.ts';

interface MacroPhrasesEditorProps {
  phrases: string[];
  phraseInput: string;
  onPhraseInputChange: (value: string) => void;
  onAddPhrase: () => void;
  onRemovePhrase: (index: number) => void;
  autoFocus?: boolean;
}

export function MacroPhrasesEditor({
  phrases,
  phraseInput,
  onPhraseInputChange,
  onAddPhrase,
  onRemovePhrase,
  autoFocus = false,
}: MacroPhrasesEditorProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '3px', flexShrink: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <label
          style={{
            fontSize: '11px',
            color: tokens.colors.textMuted,
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.4px',
          }}
        >
          Spoken Trigger Phrases (Aliases)
        </label>
        <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
          Press Enter or comma to add
        </span>
      </div>

      <div style={{ display: 'flex', gap: '4px' }}>
        <input
          type="text"
          value={phraseInput}
          onInput={(e) => onPhraseInputChange((e.target as HTMLInputElement).value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ',') {
              e.preventDefault();
              onAddPhrase();
            }
          }}
          placeholder={
            phrases.length === 0
              ? 'e.g. call airstrike (press Enter to add)'
              : 'Add another alias phrase...'
          }
          autoFocus={autoFocus}
          style={{ ...inputBaseStyle, flex: 1, padding: '5px 8px', fontSize: '12px' }}
        />
        <Button
          variant="configAction"
          onClick={onAddPhrase}
          disabled={!phraseInput.trim()}
          style={{
            padding: '4px 8px',
            fontSize: '11px',
            display: 'flex',
            alignItems: 'center',
            gap: '3px',
          }}
        >
          <IconPlus size={12} />
          <span>Add</span>
        </Button>
      </div>

      {phrases.length > 0 && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            flexWrap: 'wrap',
            marginTop: '2px',
          }}
        >
          {phrases.map((p, idx) => (
            <span
              key={idx}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 6px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 600,
                background:
                  idx === 0 ? 'rgba(88, 101, 242, 0.22)' : 'rgba(255, 255, 255, 0.08)',
                border:
                  idx === 0
                    ? '1px solid rgba(88, 101, 242, 0.45)'
                    : '1px solid rgba(255, 255, 255, 0.12)',
                color: idx === 0 ? '#9ba5ff' : tokens.colors.textSecondary,
              }}
            >
              <span>"{p}"</span>
              {idx === 0 && <span style={{ fontSize: '9px', opacity: 0.7 }}>(primary)</span>}
              <button
                type="button"
                onClick={() => onRemovePhrase(idx)}
                style={{
                  background: 'none',
                  border: 'none',
                  color: tokens.colors.textMuted,
                  cursor: 'pointer',
                  padding: '0 1px',
                  display: 'flex',
                  alignItems: 'center',
                }}
              >
                <IconX size={11} />
              </button>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
