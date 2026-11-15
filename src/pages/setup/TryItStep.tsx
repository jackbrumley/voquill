import { useSignal } from '@preact/signals';
import { IconCheck } from '@tabler/icons-preact';
import { helperTextStyle, inputBaseStyle } from '../../theme/ui-primitives.ts';
import { tokens } from '../../design-tokens.ts';
import type { SetupConfig } from './setupTypes.ts';

interface TryItStepProps {
  config: SetupConfig;
}

/// Step 4: a real dictation into a box on this page. It exercises the
/// shortcut, microphone, speech engine and typing together, so text
/// appearing here proves the whole setup works.
export function TryItStep({ config }: TryItStepProps) {
  const dictatedText = useSignal('');
  const startHow = config.hotkey_mode === 'Toggle'
    ? `press ${config.hotkey}, say a sentence, then press it again`
    : `hold ${config.hotkey}, say a sentence, then let go`;

  return (
    <>
      <div style={helperTextStyle}>Click in the box, {startHow}.</div>
      <textarea
        value={dictatedText.value}
        onInput={(event: Event) => { dictatedText.value = (event.target as HTMLTextAreaElement).value; }}
        placeholder="Your words will appear here"
        rows={3}
        aria-label="Try dictation"
        style={{ ...inputBaseStyle, width: '100%', resize: 'none', fontFamily: 'inherit' }}
      />
      {dictatedText.value.trim().length > 0 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: tokens.typography.sizeSm, color: tokens.colors.success }}>
          <IconCheck size={16} /> It works. You're ready to go.
        </div>
      )}
    </>
  );
}
