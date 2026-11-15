import { ConfigField } from '../../components/ConfigField.tsx';
import { Switch } from '../../components/Switch.tsx';
import { WordListEditor } from '../../components/WordListEditor.tsx';
import type { Config } from '../../types.ts';

interface WordsSectionProps {
  config: Config;
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
}

/// Words Voquill should get right (custom words) and words it should drop
/// (filler words).
export function WordsSection({ config, updateConfig }: WordsSectionProps) {
  return (
    <>
      <ConfigField
        label="Custom Words"
        description="Names, jargon and terms you want spelled correctly. Used by Whisper and cloud transcription; Parakeet doesn't use this list yet."
      >
        <WordListEditor
          words={config.dictionary || []}
          onChange={(words) => updateConfig('dictionary', words)}
          placeholder="e.g. Anthropic, Rust, Voquill"
        />
      </ConfigField>

      <ConfigField
        label="Remove Filler Words"
        description="Automatically remove filler words (uh, umm, hmm, etc.) from transcriptions using a fast regex pass. Works without post-processing."
      >
        <Switch
          name="Remove Filler Words"
          checked={config.filler_word_removal_enabled}
          onChange={(checked) => updateConfig('filler_word_removal_enabled', checked)}
        />
      </ConfigField>

      {config.filler_word_removal_enabled && (
        <ConfigField
          label="Custom Filler Words"
          description="Extra words to remove, like 'literally', 'actually' or 'basically'. The built-in ones (uh, umm, hmm, etc.) are always removed."
        >
          <WordListEditor
            words={config.custom_filler_words || []}
            onChange={(words) => updateConfig('custom_filler_words', words)}
            placeholder="e.g. literally, basically, actually"
            normalize={(word) => word.toLowerCase()}
          />
        </ConfigField>
      )}
    </>
  );
}
