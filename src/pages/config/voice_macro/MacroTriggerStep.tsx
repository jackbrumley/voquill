import { IconPlus, IconX, IconMicrophone, IconBulb } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { tokens } from '../../../design-tokens.ts';

interface MacroTriggerStepProps {
  phrases: string[];
  phraseInput: string;
  onPhraseInputChange: (value: string) => void;
  onAddPhrase: () => void;
  onRemovePhrase: (index: number) => void;
  autoFocus?: boolean;
}

export function MacroTriggerStep({
  phrases,
  phraseInput,
  onPhraseInputChange,
  onAddPhrase,
  onRemovePhrase,
  autoFocus = false,
}: MacroTriggerStepProps) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        flex: 1,
        minHeight: 0,
        overflowY: 'auto',
        padding: '2px',
      }}
    >
      {/* Primary Trigger Phrase Card */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '8px',
          padding: '12px',
          borderRadius: '8px',
          background: 'rgba(255, 255, 255, 0.03)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <label
            style={{
              fontSize: '11.5px',
              fontWeight: 700,
              color: '#a5b4fc',
              display: 'flex',
              alignItems: 'center',
              gap: '5px',
              textTransform: 'uppercase',
              letterSpacing: '0.4px',
            }}
          >
            <IconMicrophone size={13} />
            <span>Spoken Trigger Phrase</span>
          </label>
          <span style={{ fontSize: '10.5px', color: tokens.colors.textMuted }}>
            Enter or comma to add
          </span>
        </div>

        <div style={{ display: 'flex', gap: '6px' }}>
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
                ? 'e.g. generate a password, open terminal'
                : 'Add alternative alias phrase...'
            }
            autoFocus={autoFocus}
            style={{
              ...inputBaseStyle,
              flex: 1,
              padding: '7px 10px',
              fontSize: '12.5px',
              borderRadius: '6px',
            }}
          />
          <Button
            variant="configAction"
            onClick={onAddPhrase}
            disabled={!phraseInput.trim()}
            style={{
              padding: '6px 12px',
              fontSize: '12px',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              flexShrink: 0,
            }}
          >
            <IconPlus size={13} />
            <span>Add</span>
          </Button>
        </div>

        {/* Phrases and Aliases Tag List */}
        {phrases.length > 0 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', marginTop: '4px' }}>
            <span style={{ fontSize: '10.5px', color: tokens.colors.textMuted, fontWeight: 500 }}>
              Configured trigger phrases ({phrases.length}):
            </span>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
              {phrases.map((phrase, idx) => {
                const isPrimary = idx === 0;
                return (
                  <span
                    key={idx}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '5px',
                      padding: '4px 8px',
                      borderRadius: '5px',
                      fontSize: '11.5px',
                      fontWeight: 600,
                      background: isPrimary
                        ? 'rgba(99, 102, 241, 0.22)'
                        : 'rgba(255, 255, 255, 0.07)',
                      border: isPrimary
                        ? '1px solid rgba(99, 102, 241, 0.5)'
                        : '1px solid rgba(255, 255, 255, 0.12)',
                      color: isPrimary ? '#c7d2fe' : tokens.colors.textSecondary,
                    }}
                  >
                    <span>"{phrase}"</span>
                    {isPrimary && (
                      <span
                        style={{
                          fontSize: '9.5px',
                          padding: '1px 4px',
                          borderRadius: '3px',
                          background: 'rgba(99, 102, 241, 0.4)',
                          color: '#e0e7ff',
                        }}
                      >
                        primary
                      </span>
                    )}
                    <button
                      type="button"
                      onClick={() => onRemovePhrase(idx)}
                      title="Remove this phrase"
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
                      <IconX size={12} />
                    </button>
                  </span>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* Helpful Tips Card for New Users */}
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '6px',
          padding: '10px 12px',
          borderRadius: '8px',
          background: 'rgba(99, 102, 241, 0.07)',
          border: '1px solid rgba(99, 102, 241, 0.18)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '5px',
            fontSize: '11px',
            fontWeight: 600,
            color: '#a5b4fc',
          }}
        >
          <IconBulb size={13} />
          <span>Tips for Voice Trigger Phrases</span>
        </div>
        <ul
          style={{
            margin: 0,
            paddingLeft: '16px',
            fontSize: '11px',
            color: tokens.colors.textSecondary,
            display: 'flex',
            flexDirection: 'column',
            gap: '3px',
            lineHeight: 1.4,
          }}
        >
          <li>
            <strong>Use 2–4 words:</strong> Short, distinct commands like <em>"take screenshot"</em> work best.
          </li>
          <li>
            <strong>Add Aliases:</strong> Add common variations (e.g. <em>"grab screenshot"</em>) so it triggers naturally.
          </li>
        </ul>
      </div>
    </div>
  );
}
