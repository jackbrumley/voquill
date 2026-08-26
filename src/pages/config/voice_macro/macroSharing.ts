import type { MacroStep, VoiceMacroCommand } from '../../../types.ts';

export function generateMacroId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
}

export function cloneMacro(cmd: VoiceMacroCommand, customPhrase?: string): VoiceMacroCommand {
  const stepsCopy: MacroStep[] = (cmd.steps || []).map((step) => ({ ...step }));
  const phrasesCopy: string[] = cmd.phrases ? [...cmd.phrases] : [];
  const phrase = customPhrase || `copy of ${cmd.phrase}`;

  return {
    id: generateMacroId(),
    phrase,
    phrases: phrasesCopy,
    steps: stepsCopy,
    key_combination: null,
    hold_ms: null,
    delay_after_ms: null,
  };
}

export interface MacroExportBundle {
  voquill_version: string;
  type: 'voice_macros';
  created_at: string;
  macros: VoiceMacroCommand[];
}

export function serializeSingleMacro(cmd: VoiceMacroCommand): string {
  const clean: VoiceMacroCommand = {
    id: cmd.id,
    phrase: cmd.phrase,
    phrases: cmd.phrases ? [...cmd.phrases] : [],
    steps: cmd.steps ? cmd.steps.map((s) => ({ ...s })) : [],
    key_combination: null,
    hold_ms: null,
    delay_after_ms: null,
  };
  return JSON.stringify(clean, null, 2);
}

export function serializeMacroBundle(macros: VoiceMacroCommand[]): string {
  const bundle: MacroExportBundle = {
    voquill_version: '1.0',
    type: 'voice_macros',
    created_at: new Date().toISOString(),
    macros: macros.map((cmd) => ({
      id: cmd.id,
      phrase: cmd.phrase,
      phrases: cmd.phrases ? [...cmd.phrases] : [],
      steps: cmd.steps ? cmd.steps.map((s) => ({ ...s })) : [],
      key_combination: null,
      hold_ms: null,
      delay_after_ms: null,
    })),
  };
  return JSON.stringify(bundle, null, 2);
}

function isValidStep(step: unknown): step is MacroStep {
  if (!step || typeof step !== 'object') return false;
  const s = step as Record<string, unknown>;
  const validTypes = ['KeyPress', 'KeyDown', 'KeyUp', 'Delay', 'TypeText'];
  if (!validTypes.includes(String(s.type))) return false;

  if (s.type === 'KeyPress' || s.type === 'KeyDown' || s.type === 'KeyUp') {
    return typeof s.key === 'string' && s.key.trim().length > 0;
  }
  if (s.type === 'Delay') {
    return typeof s.duration_ms === 'number' && !isNaN(s.duration_ms) && s.duration_ms >= 0;
  }
  if (s.type === 'TypeText') {
    return typeof s.text === 'string';
  }
  return false;
}

export function parseAndValidateMacros(jsonText: string): {
  valid: VoiceMacroCommand[];
  errors: string[];
} {
  const errors: string[] = [];
  const valid: VoiceMacroCommand[] = [];

  let parsed: unknown;
  try {
    parsed = JSON.parse(jsonText);
  } catch (e) {
    return { valid: [], errors: [`Invalid JSON syntax: ${(e as Error).message}`] };
  }

  if (!parsed || typeof parsed !== 'object') {
    return { valid: [], errors: ['JSON root must be an object or an array of macros.'] };
  }

  let candidates: unknown[] = [];
  if (Array.isArray(parsed)) {
    candidates = parsed;
  } else {
    const obj = parsed as Record<string, unknown>;
    if (Array.isArray(obj.macros)) {
      candidates = obj.macros;
    } else if (typeof obj.phrase === 'string') {
      candidates = [obj];
    } else {
      return {
        valid: [],
        errors: [
          'No valid macro definitions found. Expected an object with "phrase" or a list of macros.',
        ],
      };
    }
  }

  for (let i = 0; i < candidates.length; i++) {
    const item = candidates[i];
    if (!item || typeof item !== 'object') {
      errors.push(`Item #${i + 1}: Invalid macro object format.`);
      continue;
    }

    const c = item as Record<string, unknown>;
    const phrase = typeof c.phrase === 'string' ? c.phrase.trim().toLowerCase() : '';
    if (!phrase) {
      errors.push(`Item #${i + 1}: Missing or empty trigger phrase.`);
      continue;
    }

    const phrases: string[] = [];
    if (Array.isArray(c.phrases)) {
      for (const p of c.phrases) {
        if (typeof p === 'string') {
          const clean = p.trim().toLowerCase();
          if (clean && clean !== phrase && !phrases.includes(clean)) {
            phrases.push(clean);
          }
        }
      }
    }

    let steps: MacroStep[] = [];
    if (Array.isArray(c.steps)) {
      const validSteps: MacroStep[] = [];
      let hasInvalidStep = false;
      for (let sIdx = 0; sIdx < c.steps.length; sIdx++) {
        const stepCandidate = c.steps[sIdx];
        if (isValidStep(stepCandidate)) {
          validSteps.push(stepCandidate);
        } else {
          errors.push(`Item "${phrase}" step #${sIdx + 1}: Invalid step definition.`);
          hasInvalidStep = true;
        }
      }
      if (!hasInvalidStep && validSteps.length > 0) {
        steps = validSteps;
      }
    }

    // Support legacy key_combination migration if steps array is empty
    if (steps.length === 0 && typeof c.key_combination === 'string' && c.key_combination.trim()) {
      steps = [{ type: 'KeyPress', key: c.key_combination.trim(), hold_ms: 50 }];
    }

    if (steps.length === 0) {
      errors.push(`Item "${phrase}": Macro must have at least one valid execution step.`);
      continue;
    }

    valid.push({
      id: generateMacroId(),
      phrase,
      phrases,
      steps,
      key_combination: null,
      hold_ms: null,
      delay_after_ms: null,
    });
  }

  return { valid, errors };
}

export function sanitizeImportedPhrases(
  imported: VoiceMacroCommand[],
  existing: VoiceMacroCommand[]
): VoiceMacroCommand[] {
  const existingPhrases = new Set(existing.map((m) => m.phrase.trim().toLowerCase()));

  return imported.map((cmd) => {
    let phrase = cmd.phrase;
    if (existingPhrases.has(phrase)) {
      let counter = 2;
      while (existingPhrases.has(`${phrase} (${counter})`)) {
        counter++;
      }
      phrase = `${phrase} (${counter})`;
    }
    existingPhrases.add(phrase);

    return {
      ...cmd,
      id: generateMacroId(),
      phrase,
    };
  });
}
