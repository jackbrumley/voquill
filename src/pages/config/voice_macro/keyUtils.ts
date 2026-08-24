import type { MacroStep, VoiceMacroCommand } from '../../../types.ts';

export function normalizeKeyName(e: KeyboardEvent): string {
  const code = e.code;
  if (code.startsWith('Key')) {
    return code.substring(3).toUpperCase();
  }
  if (code.startsWith('Digit')) {
    return code.substring(5);
  }
  if (code.startsWith('Numpad') && code.length === 7 && !isNaN(Number(code.charAt(6)))) {
    return code.substring(6);
  }
  if (/^F\d{1,2}$/i.test(code)) {
    return code.toUpperCase();
  }
  switch (code) {
    case 'ControlLeft':
    case 'ControlRight':
      return 'Ctrl';
    case 'ShiftLeft':
    case 'ShiftRight':
      return 'Shift';
    case 'AltLeft':
    case 'AltRight':
      return 'Alt';
    case 'MetaLeft':
    case 'MetaRight':
    case 'OSLeft':
    case 'OSRight':
      return 'Super';
    case 'Space':
      return 'Space';
    case 'Enter':
    case 'NumpadEnter':
      return 'Enter';
    case 'Tab':
      return 'Tab';
    case 'Escape':
      return 'Escape';
    case 'Backspace':
      return 'Backspace';
    case 'Delete':
      return 'Delete';
    case 'Insert':
      return 'Insert';
    case 'Home':
      return 'Home';
    case 'End':
      return 'End';
    case 'PageUp':
      return 'PageUp';
    case 'PageDown':
      return 'PageDown';
    case 'ArrowUp':
      return 'ArrowUp';
    case 'ArrowDown':
      return 'ArrowDown';
    case 'ArrowLeft':
      return 'ArrowLeft';
    case 'ArrowRight':
      return 'ArrowRight';
    case 'Comma':
      return ',';
    case 'Period':
      return '.';
    case 'Slash':
      return '/';
    case 'Semicolon':
      return ';';
    case 'Quote':
      return '\'';
    case 'BracketLeft':
      return '[';
    case 'BracketRight':
      return ']';
    case 'Backslash':
      return '\\';
    case 'Minus':
      return '-';
    case 'Equal':
      return '=';
    case 'Backquote':
      return '`';
    default:
      return e.key.length === 1 ? e.key.toUpperCase() : e.key;
  }
}

export function resolveMacroSteps(cmd: VoiceMacroCommand): MacroStep[] {
  if (cmd.steps && cmd.steps.length > 0) {
    return cmd.steps;
  }
  if (cmd.key_combination && cmd.key_combination.trim()) {
    const hold = cmd.hold_ms ?? 50;
    const steps: MacroStep[] = [{ type: 'KeyPress', key: cmd.key_combination, hold_ms: hold }];
    if (cmd.delay_after_ms && cmd.delay_after_ms > 0) {
      steps.push({ type: 'Delay', duration_ms: cmd.delay_after_ms });
    }
    return steps;
  }
  return [];
}
