import { useSignal } from '@preact/signals';
import {
  IconPlus,
  IconKeyboard,
  IconWriting,
  IconTerminal2,
  IconClock,
} from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { tokens } from '../../../design-tokens.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import type { useMacroSequence } from './useMacroSequence.ts';

type ManualActionTab = 'key' | 'text' | 'command';

interface MacroManualActionBarProps {
  sequence: ReturnType<typeof useMacroSequence>;
}

export function MacroManualActionBar({ sequence }: MacroManualActionBarProps) {
  const manualTab = useSignal<ManualActionTab>('key');
  const {
    manualKeyInput,
    manualTextInput,
    manualCommandInput,
    addManualKey,
    addManualDelay,
    addManualText,
    addManualCommand,
  } = sequence;

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '6px',
        padding: '8px 10px',
        borderRadius: '7px',
        background: 'rgba(255, 255, 255, 0.02)',
        border: '1px solid rgba(255, 255, 255, 0.07)',
        flexShrink: 0,
      }}
    >
      {/* Subtabs for Manual Type */}
      <div style={{ display: 'flex', gap: '4px', alignItems: 'center' }}>
        <span
          style={{
            fontSize: '10.5px',
            fontWeight: 600,
            color: tokens.colors.textMuted,
            textTransform: 'uppercase',
            marginRight: '2px',
          }}
        >
          Add:
        </span>
        <button
          type="button"
          onClick={() => {
            manualTab.value = 'key';
          }}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '3px 8px',
            borderRadius: '4px',
            fontSize: '11px',
            fontWeight: manualTab.value === 'key' ? 600 : 500,
            background:
              manualTab.value === 'key' ? 'rgba(88, 101, 242, 0.22)' : 'transparent',
            border:
              manualTab.value === 'key'
                ? '1px solid rgba(88, 101, 242, 0.45)'
                : '1px solid transparent',
            color: manualTab.value === 'key' ? '#a5b4fc' : tokens.colors.textSecondary,
            cursor: 'pointer',
          }}
        >
          <IconKeyboard size={12} />
          <span>Keys & Delays</span>
        </button>
        <button
          type="button"
          onClick={() => {
            manualTab.value = 'text';
          }}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '3px 8px',
            borderRadius: '4px',
            fontSize: '11px',
            fontWeight: manualTab.value === 'text' ? 600 : 500,
            background:
              manualTab.value === 'text' ? 'rgba(88, 101, 242, 0.22)' : 'transparent',
            border:
              manualTab.value === 'text'
                ? '1px solid rgba(88, 101, 242, 0.45)'
                : '1px solid transparent',
            color: manualTab.value === 'text' ? '#a5b4fc' : tokens.colors.textSecondary,
            cursor: 'pointer',
          }}
        >
          <IconWriting size={12} />
          <span>Type Text</span>
        </button>
        <button
          type="button"
          onClick={() => {
            manualTab.value = 'command';
          }}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '3px 8px',
            borderRadius: '4px',
            fontSize: '11px',
            fontWeight: manualTab.value === 'command' ? 600 : 500,
            background:
              manualTab.value === 'command' ? 'rgba(88, 101, 242, 0.22)' : 'transparent',
            border:
              manualTab.value === 'command'
                ? '1px solid rgba(88, 101, 242, 0.45)'
                : '1px solid transparent',
            color: manualTab.value === 'command' ? '#a5b4fc' : tokens.colors.textSecondary,
            cursor: 'pointer',
          }}
        >
          <IconTerminal2 size={12} />
          <span>Command</span>
        </button>
      </div>

      {/* Key & Delay Controls */}
      {manualTab.value === 'key' && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap' }}>
          <input
            type="text"
            value={manualKeyInput.value}
            onInput={(e) => {
              manualKeyInput.value = (e.target as HTMLInputElement).value;
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                addManualKey('KeyPress');
              }
            }}
            placeholder="Key (e.g. F3, Enter, Ctrl)"
            style={{
              ...inputBaseStyle,
              flex: '1 1 110px',
              minWidth: '100px',
              padding: '4px 8px',
              fontSize: '11.5px',
            }}
          />
          <Button
            variant="configAction"
            onClick={() => addManualKey('KeyPress')}
            disabled={!manualKeyInput.value.trim()}
            title="Tap (Press and release key)"
            style={{ padding: '4px 8px', fontSize: '11px' }}
          >
            + Tap
          </Button>
          <Button
            variant="configAction"
            onClick={() => addManualKey('KeyDown')}
            disabled={!manualKeyInput.value.trim()}
            title="Hold key down"
            style={{ padding: '4px 8px', fontSize: '11px' }}
          >
            + Hold
          </Button>
          <Button
            variant="configAction"
            onClick={() => addManualKey('KeyUp')}
            disabled={!manualKeyInput.value.trim()}
            title="Release key"
            style={{ padding: '4px 8px', fontSize: '11px' }}
          >
            + Rel
          </Button>
          <Button
            variant="configAction"
            onClick={addManualDelay}
            title="Insert a 100ms pause"
            style={{
              padding: '4px 8px',
              fontSize: '11px',
              display: 'flex',
              alignItems: 'center',
              gap: '3px',
            }}
          >
            <IconClock size={11} />
            <span>+ 100ms</span>
          </Button>
        </div>
      )}

      {/* Type Text Controls */}
      {manualTab.value === 'text' && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
          <input
            type="text"
            value={manualTextInput.value}
            onInput={(e) => {
              manualTextInput.value = (e.target as HTMLInputElement).value;
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                addManualText();
              }
            }}
            placeholder="Type text string to output..."
            style={{ ...inputBaseStyle, flex: 1, padding: '4px 8px', fontSize: '11.5px' }}
          />
          <Button
            variant="configAction"
            onClick={addManualText}
            disabled={!manualTextInput.value.trim()}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              flexShrink: 0,
            }}
          >
            <IconPlus size={12} />
            <span>Insert Text</span>
          </Button>
        </div>
      )}

      {/* Run Command Controls */}
      {manualTab.value === 'command' && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
          <input
            type="text"
            value={manualCommandInput.value}
            onInput={(e) => {
              manualCommandInput.value = (e.target as HTMLInputElement).value;
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                addManualCommand();
              }
            }}
            placeholder="Run system command / script..."
            style={{
              ...inputBaseStyle,
              flex: 1,
              padding: '4px 8px',
              fontSize: '11px',
              fontFamily: 'monospace',
            }}
          />
          <Button
            variant="configAction"
            onClick={addManualCommand}
            disabled={!manualCommandInput.value.trim()}
            style={{
              padding: '4px 10px',
              fontSize: '11px',
              display: 'flex',
              alignItems: 'center',
              gap: '4px',
              flexShrink: 0,
            }}
          >
            <IconPlus size={12} />
            <span>Add Command</span>
          </Button>
        </div>
      )}
    </div>
  );
}
