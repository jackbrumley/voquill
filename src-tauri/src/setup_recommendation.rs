//! The speech engine Voquill recommends during initial setup, chosen from
//! the hardware it detects and the configured language. The rule lives here
//! so the setup screen only renders the result.
//!
//! Tiers, best first:
//! 1. A suitable graphics card: Parakeet on the GPU, with the processor
//!    choice offered second.
//! 2. A modest computer (under 8 GB of memory or 4 processor threads or
//!    fewer), English: the light Whisper model first, Parakeet second.
//! 3. Otherwise: Parakeet on the processor.
//!
//! English (or automatic) languages get English models, which are the most
//! accurate for English; any other language gets the multilingual ones.

use crate::engines::ModelEngine;
use crate::hardware_detection::{GraphicsDevice, GraphicsKind, HardwareProfile};
use crate::model_manager::ModelManager;
use serde::Serialize;

const GIGABYTE: u64 = 1024 * 1024 * 1024;
/// Graphics memory needed to run the GPU model comfortably (it uses about
/// 1.5 GB, plus room for the desktop and other apps).
const MIN_GPU_MEMORY_BYTES: u64 = 4 * GIGABYTE;
/// Below this much system memory the 0.6B processor model (1.5-2 GB at its
/// peak) crowds out other apps.
const MODEST_MEMORY_BYTES: u64 = 8 * GIGABYTE;
/// At or below this many processor threads the 0.6B model is noticeably slow.
const MODEST_PROCESSOR_THREADS: usize = 4;

/// The models each tier uses, by language family.
struct ModelChoices {
    gpu: (ModelEngine, &'static str),
    processor: (ModelEngine, &'static str),
    /// No light multilingual model is in the catalog.
    light: Option<(ModelEngine, &'static str)>,
    language_phrase: &'static str,
}

const ENGLISH_MODELS: ModelChoices = ModelChoices {
    gpu: (ModelEngine::ParakeetGpu, "tdt-0.6b-v2-q8_0"),
    processor: (ModelEngine::ParakeetCpu, "parakeet-unified-en-0.6b"),
    light: Some((ModelEngine::WhisperCpu, "distil-small.en")),
    language_phrase: "English",
};

const MULTILINGUAL_MODELS: ModelChoices = ModelChoices {
    gpu: (ModelEngine::ParakeetGpu, "tdt-0.6b-v3-q8_0"),
    processor: (ModelEngine::ParakeetCpu, "parakeet-tdt-0.6b-v3"),
    light: None,
    language_phrase: "multilingual",
};

/// The engine and model for settings created before any hardware has been
/// checked: the "works on any computer" choice for the default (automatic,
/// treated as English) language. Initial setup replaces it with the
/// recommendation for the actual hardware.
pub fn baseline_choice() -> (ModelEngine, &'static str) {
    ENGLISH_MODELS.processor
}

#[derive(Debug, Serialize)]
pub struct SetupOption {
    pub engine: ModelEngine,
    pub model_size: String,
    pub title: String,
    pub description: String,
    pub download_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct SetupRecommendation {
    /// The graphics card the GPU option would use, if one is suitable.
    pub graphics_card: Option<String>,
    /// Choices in order of preference; the first is the recommendation.
    pub options: Vec<SetupOption>,
}

/// Builds the recommendation for `hardware` and the configured `language`.
pub fn recommend(hardware: &HardwareProfile, language: &str) -> SetupRecommendation {
    let models = if is_english(language) {
        &ENGLISH_MODELS
    } else {
        &MULTILINGUAL_MODELS
    };
    let graphics_card = if crate::parakeet::gpu_backend_available() {
        suitable_graphics_card(&hardware.graphics).map(|device| device.description.clone())
    } else {
        None
    };

    let mut options = Vec::new();
    if let Some(card) = &graphics_card {
        options.extend(option(
            models.gpu,
            "Best quality, uses your graphics card",
            format!(
                "Fast, accurate {} dictation on your {}.",
                models.language_phrase, card
            ),
        ));
        options.extend(option(
            models.processor,
            "Works on any computer",
            format!(
                "Accurate {} dictation on your processor. Slower.",
                models.language_phrase
            ),
        ));
    } else if let (true, Some(light)) = (is_modest(hardware), models.light) {
        options.extend(option(
            light,
            "Light, for this computer",
            format!(
                "Quick {} dictation that stays light on memory.",
                models.language_phrase
            ),
        ));
        options.extend(option(
            models.processor,
            "More accurate, needs a faster computer",
            format!(
                "Best {} accuracy on your processor, but slower here.",
                models.language_phrase
            ),
        ));
    } else {
        options.extend(option(
            models.processor,
            "Works on any computer",
            format!(
                "Accurate {} dictation on your processor.",
                models.language_phrase
            ),
        ));
    }

    crate::log_info!(
        "Setup recommendation: language {}, graphics card {:?}, modest {}, first choice {:?}",
        language,
        graphics_card,
        is_modest(hardware),
        options.first().map(|choice| &choice.model_size)
    );
    SetupRecommendation {
        graphics_card,
        options,
    }
}

/// "auto" counts as English: it is the default, and the English models
/// handle English best.
fn is_english(language: &str) -> bool {
    language == "auto" || language.starts_with("en")
}

/// Whether the processor model would be heavy here. Unknown memory (0) is
/// not treated as modest.
fn is_modest(hardware: &HardwareProfile) -> bool {
    let low_memory =
        hardware.system_memory_bytes > 0 && hardware.system_memory_bytes < MODEST_MEMORY_BYTES;
    low_memory || hardware.processor_threads <= MODEST_PROCESSOR_THREADS
}

/// The card with the most memory among those good enough for the GPU model.
/// Integrated graphics share slow system memory and are skipped, except on
/// Apple Silicon, where the integrated GPU is the fast path.
fn suitable_graphics_card(devices: &[GraphicsDevice]) -> Option<&GraphicsDevice> {
    let integrated_is_fast = cfg!(all(target_os = "macos", target_arch = "aarch64"));
    devices
        .iter()
        .filter(|device| device.kind == GraphicsKind::Discrete || integrated_is_fast)
        .filter(|device| device.total_memory_bytes >= MIN_GPU_MEMORY_BYTES)
        .max_by_key(|device| device.total_memory_bytes)
}

/// One choice, sized from the model catalog (the single source of model
/// facts). A model missing from the catalog is logged and left out.
fn option(
    (engine, model_size): (ModelEngine, &str),
    title: &str,
    description: String,
) -> Option<SetupOption> {
    let Some(model) = ModelManager::find_model(engine, model_size) else {
        crate::log_warn!(
            "Setup recommendation names {} / {}, which is not in the model catalog",
            engine.name(),
            model_size
        );
        return None;
    };
    Some(SetupOption {
        engine,
        model_size: model_size.to_string(),
        title: title.to_string(),
        description,
        download_bytes: model.file_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(kind: GraphicsKind, gigabytes: u64) -> GraphicsDevice {
        GraphicsDevice {
            description: format!("{:?} {} GB", kind, gigabytes),
            kind,
            total_memory_bytes: gigabytes * GIGABYTE,
        }
    }

    fn hardware(
        graphics: Vec<GraphicsDevice>,
        memory_gigabytes: u64,
        threads: usize,
    ) -> HardwareProfile {
        HardwareProfile {
            graphics,
            system_memory_bytes: memory_gigabytes * GIGABYTE,
            processor_threads: threads,
        }
    }

    fn first_model(recommendation: &SetupRecommendation) -> &str {
        &recommendation.options[0].model_size
    }

    #[test]
    fn every_recommended_model_exists_in_the_catalog() {
        for models in [&ENGLISH_MODELS, &MULTILINGUAL_MODELS] {
            let mut choices = vec![models.gpu, models.processor];
            choices.extend(models.light);
            for (engine, size) in choices {
                assert!(
                    ModelManager::find_model(engine, size).is_some(),
                    "{} / {size}",
                    engine.name()
                );
            }
        }
    }

    #[test]
    fn picks_the_discrete_card_with_the_most_memory() {
        let devices = [
            device(GraphicsKind::Discrete, 8),
            device(GraphicsKind::Discrete, 20),
            device(GraphicsKind::Integrated, 32),
        ];
        let card = suitable_graphics_card(&devices).unwrap();
        assert_eq!(card.total_memory_bytes, 20 * GIGABYTE);
    }

    #[test]
    fn small_cards_are_not_suitable() {
        assert!(suitable_graphics_card(&[device(GraphicsKind::Discrete, 2)]).is_none());
    }

    #[test]
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    fn laptop_with_integrated_graphics_gets_the_processor_model() {
        let laptop = hardware(vec![device(GraphicsKind::Integrated, 16)], 16, 8);
        let recommendation = recommend(&laptop, "en-AU");
        assert!(recommendation.graphics_card.is_none());
        assert_eq!(first_model(&recommendation), ENGLISH_MODELS.processor.1);
    }

    #[test]
    fn modest_computer_gets_the_light_model_first() {
        let low_memory = hardware(vec![], 4, 8);
        assert_eq!(
            first_model(&recommend(&low_memory, "auto")),
            "distil-small.en"
        );
        let few_threads = hardware(vec![], 16, 4);
        assert_eq!(
            first_model(&recommend(&few_threads, "auto")),
            "distil-small.en"
        );
        assert_eq!(recommend(&few_threads, "auto").options.len(), 2);
    }

    #[test]
    fn unknown_memory_is_not_modest() {
        assert!(!is_modest(&hardware(vec![], 0, 8)));
    }

    #[test]
    fn other_languages_get_multilingual_models() {
        let modest = hardware(vec![], 4, 2);
        let recommendation = recommend(&modest, "de");
        assert_eq!(
            first_model(&recommendation),
            MULTILINGUAL_MODELS.processor.1
        );
        assert_eq!(recommendation.options.len(), 1);
    }
}
