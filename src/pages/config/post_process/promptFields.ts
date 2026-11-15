import type { Config, PostProcessPrompt } from '../../../types.ts';
import { backendDefaultConfig } from '../../../defaultConfig.ts';

/// The three prompt values a saved custom prompt can override. Mirrors
/// `Config::resolve_post_process_prompt`, `resolve_user_prompt_template` and
/// `resolve_max_output_tokens` in `config/post_process.rs`, which decide what
/// a dictation uses; this only decides which value the editor shows and
/// edits.
export type PromptField = 'prompt' | 'user_prompt_template' | 'max_output_tokens';

type PromptValue<F extends PromptField> = F extends 'max_output_tokens' ? number : string;

const DEFAULT_KEYS = {
  prompt: 'post_process_prompt',
  user_prompt_template: 'post_process_user_prompt_template',
  max_output_tokens: 'post_process_max_output_tokens',
} as const;

function selectedPrompt(config: Config) {
  const selectedId = config.post_process_selected_prompt_id;
  return selectedId ? (config.post_process_prompts || []).find((prompt) => prompt.id === selectedId) : undefined;
}

/// The value the selected prompt uses: its own override, else the default.
export function resolvePromptField<F extends PromptField>(config: Config, field: F): PromptValue<F> {
  const fallback = config[DEFAULT_KEYS[field]] as PromptValue<F>;
  const selected = selectedPrompt(config);
  if (!selected) {
    return fallback;
  }
  const override = selected[field];
  if (override === undefined || override === null) {
    return fallback;
  }
  if (field === 'max_output_tokens' && override === 0) {
    return fallback;
  }
  return override as PromptValue<F>;
}

/// The settings change that stores `value`: on the selected prompt when one
/// is chosen, otherwise on the default.
export function promptFieldUpdate<F extends PromptField>(
  config: Config,
  field: F,
  value: PromptValue<F>,
): [key: string, value: string | number | PostProcessPrompt[]] {
  const selected = selectedPrompt(config);
  if (!selected) {
    return [DEFAULT_KEYS[field], value];
  }
  return [
    'post_process_prompts',
    (config.post_process_prompts || []).map((prompt) => (prompt.id === selected.id ? { ...prompt, [field]: value } : prompt)),
  ];
}

/// The built-in value of a field, from the backend's defaults.
export function defaultPromptField<F extends PromptField>(field: F): PromptValue<F> {
  return backendDefaultConfig()[DEFAULT_KEYS[field]] as PromptValue<F>;
}
