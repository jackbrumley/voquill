//! Voice macro commands as stored in the settings file.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum MacroStep {
    KeyPress {
        key: String,
        #[serde(default = "default_macro_step_hold_ms")]
        hold_ms: u64,
    },
    KeyDown {
        key: String,
    },
    KeyUp {
        key: String,
    },
    Delay {
        duration_ms: u64,
    },
    TypeText {
        text: String,
    },
    RunCommand {
        command: String,
    },
}

/// How long a macro key press is held when the macro does not say.
fn default_macro_step_hold_ms() -> u64 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MacroSoundMode {
    #[default]
    Default,
    None,
    Tts,
    CustomFile,
    MicRecording,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceMacroCommand {
    pub id: String,
    pub phrase: String,
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default)]
    pub steps: Vec<MacroStep>,
    #[serde(default)]
    pub key_combination: Option<String>,
    #[serde(default)]
    pub hold_ms: Option<u64>,
    #[serde(default)]
    pub delay_after_ms: Option<u64>,
    #[serde(default)]
    pub sound_mode: MacroSoundMode,
    #[serde(default)]
    pub sound_tts_text: Option<String>,
    #[serde(default)]
    pub sound_tts_voice: Option<String>,
    #[serde(default)]
    pub sound_tts_speed: Option<f32>,
    #[serde(default)]
    pub sound_tts_effect: Option<String>,
    #[serde(default)]
    pub sound_tts_pitch: Option<f32>,
}

impl VoiceMacroCommand {
    pub fn all_phrases(&self) -> Vec<String> {
        let mut list = Vec::new();
        let primary = self.phrase.trim();
        if !primary.is_empty() {
            list.push(primary.to_string());
        }
        for p in &self.phrases {
            let clean = p.trim();
            if !clean.is_empty()
                && !list
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(clean))
            {
                list.push(clean.to_string());
            }
        }
        list
    }

    pub fn resolve_steps(&self) -> Vec<MacroStep> {
        if !self.steps.is_empty() {
            return self.steps.clone();
        }
        if let Some(ref combo) = self.key_combination {
            if !combo.trim().is_empty() {
                let hold = self.hold_ms.unwrap_or_else(default_macro_step_hold_ms);
                let mut steps = vec![MacroStep::KeyPress {
                    key: combo.clone(),
                    hold_ms: hold,
                }];
                if let Some(delay) = self.delay_after_ms {
                    if delay > 0 {
                        steps.push(MacroStep::Delay { duration_ms: delay });
                    }
                }
                return steps;
            }
        }
        Vec::new()
    }
}
