//! The single owner of engine identity: every local engine Voquill can run,
//! its display name (which is also what `settings.json` stores and the UI
//! shows), whether it runs on the graphics card, its family and its model
//! category. Nothing else matches engine names as text.
//!
//! Settings keep storing engines as their name string and are parsed here,
//! so an old or unknown name never makes the settings file unreadable; it
//! surfaces as a clear error where the engine is used.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelEngine {
    #[serde(rename = "Whisper.cpp")]
    WhisperCpu,
    #[serde(rename = "Whisper.cpp (GPU)")]
    WhisperGpu,
    #[serde(rename = "Parakeet")]
    ParakeetCpu,
    #[serde(rename = "Parakeet (GPU)")]
    ParakeetGpu,
    #[serde(rename = "Post-Process (Local)")]
    PostProcessCpu,
    #[serde(rename = "Post-Process (GPU)")]
    PostProcessGpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineFamily {
    Whisper,
    Parakeet,
    PostProcess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCategory {
    Transcription,
    PostProcess,
}

impl ModelEngine {
    pub const ALL: [ModelEngine; 6] = [
        ModelEngine::WhisperCpu,
        ModelEngine::WhisperGpu,
        ModelEngine::ParakeetCpu,
        ModelEngine::ParakeetGpu,
        ModelEngine::PostProcessCpu,
        ModelEngine::PostProcessGpu,
    ];

    /// The name stored in settings and shown in the UI.
    pub fn name(self) -> &'static str {
        match self {
            ModelEngine::WhisperCpu => "Whisper.cpp",
            ModelEngine::WhisperGpu => "Whisper.cpp (GPU)",
            ModelEngine::ParakeetCpu => "Parakeet",
            ModelEngine::ParakeetGpu => "Parakeet (GPU)",
            ModelEngine::PostProcessCpu => "Post-Process (Local)",
            ModelEngine::PostProcessGpu => "Post-Process (GPU)",
        }
    }

    pub fn parse(name: &str) -> Option<ModelEngine> {
        ModelEngine::ALL
            .into_iter()
            .find(|engine| engine.name() == name)
    }

    /// Parses a stored engine name, failing with a message that lists the
    /// engines that do exist.
    pub fn parse_required(name: &str) -> Result<ModelEngine, String> {
        ModelEngine::parse(name).ok_or_else(|| {
            let known: Vec<&str> = ModelEngine::ALL.iter().map(|e| e.name()).collect();
            format!(
                "Unknown engine '{}'. Known engines: {}",
                name,
                known.join(", ")
            )
        })
    }

    pub fn uses_gpu(self) -> bool {
        matches!(
            self,
            ModelEngine::WhisperGpu | ModelEngine::ParakeetGpu | ModelEngine::PostProcessGpu
        )
    }

    pub fn family(self) -> EngineFamily {
        match self {
            ModelEngine::WhisperCpu | ModelEngine::WhisperGpu => EngineFamily::Whisper,
            ModelEngine::ParakeetCpu | ModelEngine::ParakeetGpu => EngineFamily::Parakeet,
            ModelEngine::PostProcessCpu | ModelEngine::PostProcessGpu => EngineFamily::PostProcess,
        }
    }

    /// Whether the engine uses the custom words list (passed as Whisper's
    /// prompt hint). Parakeet ignores it.
    pub fn uses_custom_words(self) -> bool {
        self.family() == EngineFamily::Whisper
    }

    pub fn category(self) -> ModelCategory {
        match self.family() {
            EngineFamily::Whisper | EngineFamily::Parakeet => ModelCategory::Transcription,
            EngineFamily::PostProcess => ModelCategory::PostProcess,
        }
    }
}

/// What the UI needs to know about an engine, sent by `get_available_engines`.
#[derive(Clone, Debug, Serialize)]
pub struct EngineInfo {
    pub name: &'static str,
    pub uses_gpu: bool,
    pub uses_custom_words: bool,
    pub category: ModelCategory,
}

impl From<ModelEngine> for EngineInfo {
    fn from(engine: ModelEngine) -> Self {
        EngineInfo {
            name: engine.name(),
            uses_gpu: engine.uses_gpu(),
            uses_custom_words: engine.uses_custom_words(),
            category: engine.category(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for engine in ModelEngine::ALL {
            assert_eq!(ModelEngine::parse(engine.name()), Some(engine));
        }
    }

    #[test]
    fn serde_uses_the_stored_names() {
        assert_eq!(
            serde_json::to_string(&ModelEngine::ParakeetGpu).unwrap(),
            "\"Parakeet (GPU)\""
        );
    }

    #[test]
    fn unknown_names_are_rejected_with_the_known_list() {
        let error = ModelEngine::parse_required("Vosk").unwrap_err();
        assert!(error.contains("Whisper.cpp"));
    }

    #[test]
    fn gpu_engines_are_flagged() {
        assert!(ModelEngine::WhisperGpu.uses_gpu());
        assert!(!ModelEngine::ParakeetCpu.uses_gpu());
        assert_eq!(
            ModelEngine::PostProcessGpu.category(),
            ModelCategory::PostProcess
        );
        assert!(ModelEngine::WhisperGpu.uses_custom_words());
        assert!(!ModelEngine::ParakeetGpu.uses_custom_words());
    }
}
