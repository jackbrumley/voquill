import { IconPlus } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';

interface MacroManualAddersProps {
  manualKeyInput: string;
  onManualKeyInputChange: (value: string) => void;
  manualTextInput: string;
  onManualTextInputChange: (value: string) => void;
  onAddManualKey: (type: 'KeyPress' | 'KeyDown' | 'KeyUp') => void;
  onAddManualDelay: () => void;
  onAddManualText: () => void;
}

export function MacroManualAdders({
  manualKeyInput,
  onManualKeyInputChange,
  manualTextInput,
  onManualTextInputChange,
  onAddManualKey,
  onAddManualDelay,
  onAddManualText,
}: MacroManualAddersProps) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '6px',
        padding: '8px',
        borderRadius: '6px',
        background: 'rgba(255, 255, 255, 0.02)',
        border: '1px solid rgba(255, 255, 255, 0.06)',
        flexShrink: 0,
      }}
    >
      {/* Key Action Adders */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap' }}>
        <input
          type="text"
          value={manualKeyInput}
          onInput={(e) => onManualKeyInputChange((e.target as HTMLInputElement).value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              onAddManualKey('KeyPress');
            }
          }}
          placeholder="Key (F3, W, Ctrl)"
          style={{ ...inputBaseStyle, width: '100px', padding: '3px 6px', fontSize: '11px' }}
        />
        <Button
          variant="configAction"
          onClick={() => onAddManualKey('KeyPress')}
          style={{ padding: '3px 6px', fontSize: '10.5px' }}
        >
          + Tap
        </Button>
        <Button
          variant="configAction"
          onClick={() => onAddManualKey('KeyDown')}
          style={{ padding: '3px 6px', fontSize: '10.5px' }}
        >
          + Hold
        </Button>
        <Button
          variant="configAction"
          onClick={() => onAddManualKey('KeyUp')}
          style={{ padding: '3px 6px', fontSize: '10.5px' }}
        >
          + Rel
        </Button>
        <Button
          variant="configAction"
          onClick={onAddManualDelay}
          style={{ padding: '3px 6px', fontSize: '10.5px' }}
        >
          + 100ms
        </Button>
      </div>

      {/* Type Text Adder */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
        <input
          type="text"
          value={manualTextInput}
          onInput={(e) => onManualTextInputChange((e.target as HTMLInputElement).value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              onAddManualText();
            }
          }}
          placeholder="Type text string..."
          style={{ ...inputBaseStyle, flex: 1, padding: '3px 6px', fontSize: '11px' }}
        />
        <Button
          variant="configAction"
          onClick={onAddManualText}
          style={{
            padding: '3px 6px',
            fontSize: '10.5px',
            display: 'flex',
            alignItems: 'center',
            gap: '3px',
          }}
        >
          <IconPlus size={11} />
          <span>+ Text</span>
        </Button>
      </div>
    </div>
  );
}
