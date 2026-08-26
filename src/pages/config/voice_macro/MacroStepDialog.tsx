import { useSignal } from '@preact/signals';
import { useEffect, useRef } from 'preact/hooks';
import { IconWriting, IconTerminal2, IconCheck, IconX } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { tokens } from '../../../design-tokens.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';

interface MacroStepDialogProps {
  stepIndex: number;
  stepType: 'TypeText' | 'RunCommand';
  initialValue: string;
  onSave: (value: string) => void;
  onCancel: () => void;
}

export function MacroStepDialog({
  stepIndex,
  stepType,
  initialValue,
  onSave,
  onCancel,
}: MacroStepDialogProps) {
  const textValue = useSignal(initialValue);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const isCommand = stepType === 'RunCommand';

  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.focus();
      textarea.select();
    }
  }, []);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();
      if (textValue.value.trim()) {
        onSave(textValue.value);
      }
    }
  };

  const handleSave = () => {
    if (textValue.value.trim()) {
      onSave(textValue.value);
    }
  };

  return (
    <div
      onClick={onCancel}
      style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        zIndex: 50,
        background: 'rgba(0, 0, 0, 0.65)',
        backdropFilter: 'blur(2px)',
        WebkitBackdropFilter: 'blur(2px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '16px',
        animation: 'fadeIn 0.15s ease',
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: '100%',
          maxWidth: '340px',
          background: tokens.colors.bgSecondary,
          border: '1px solid rgba(255, 255, 255, 0.14)',
          borderRadius: '10px',
          padding: '14px',
          display: 'flex',
          flexDirection: 'column',
          gap: '10px',
          boxShadow: tokens.shadows.lg,
        }}
      >
        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '2px 7px',
                borderRadius: '4px',
                fontSize: '11px',
                fontWeight: 700,
                background: isCommand ? 'rgba(236, 72, 153, 0.2)' : 'rgba(16, 185, 129, 0.2)',
                color: isCommand ? '#f472b6' : '#34d399',
                border: isCommand
                  ? '1px solid rgba(236, 72, 153, 0.4)'
                  : '1px solid rgba(16, 185, 129, 0.4)',
              }}
            >
              {isCommand ? <IconTerminal2 size={12} /> : <IconWriting size={12} />}
              <span>{isCommand ? 'Command' : 'Text Output'}</span>
            </span>
            <span
              style={{
                fontSize: '11.5px',
                fontWeight: 600,
                color: tokens.colors.textPrimary,
              }}
            >
              Step {stepIndex + 1}
            </span>
          </div>

          <button
            type="button"
            onClick={onCancel}
            title="Cancel"
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              cursor: 'pointer',
              padding: '2px',
              display: 'flex',
              alignItems: 'center',
            }}
          >
            <IconX size={15} />
          </button>
        </div>

        {/* Textarea */}
        <textarea
          ref={textareaRef}
          value={textValue.value}
          onInput={(e) => {
            textValue.value = (e.target as HTMLTextAreaElement).value;
          }}
          onKeyDown={handleKeyDown}
          placeholder={
            isCommand
              ? 'Enter bash command or CLI script...'
              : 'Enter text string to type out...'
          }
          rows={4}
          style={{
            ...inputBaseStyle,
            minHeight: '80px',
            maxHeight: '180px',
            resize: 'vertical',
            padding: '8px 10px',
            fontSize: isCommand ? '11px' : '12px',
            fontFamily: isCommand ? tokens.typography.fontMono : undefined,
            lineHeight: 1.4,
          }}
        />

        {/* Shortcut Hint & Actions */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            paddingTop: '2px',
          }}
        >
          <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
            Press <strong>Ctrl+Enter</strong> to save
          </span>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Button
              variant="ghost"
              onClick={onCancel}
              style={{ padding: '4px 10px', fontSize: '11.5px' }}
            >
              Cancel
            </Button>
            <Button
              variant="configAction"
              onClick={handleSave}
              disabled={!textValue.value.trim()}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                padding: '4px 12px',
                fontSize: '11.5px',
                fontWeight: 600,
              }}
            >
              <IconCheck size={13} />
              <span>Save Step</span>
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
