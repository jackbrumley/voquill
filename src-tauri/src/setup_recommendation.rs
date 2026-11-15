//! The speech engine Voquill recommends during initial setup, chosen from
//! the hardware it detects. The rule lives here so the setup screen only
//! renders the result.

use crate::gpu_detection::{GraphicsDevice, GraphicsKind};
use crate::model_manager::ModelManager;
use serde::Serialize;

/// Graphics memory needed to run the GPU model comfortably (it uses about
/// 1.5 GB, plus room for the desktop and other apps).
const MIN_GPU_MEMORY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

const GPU_ENGINE: &str = "Parakeet (GPU)";
const GPU_MODEL: &str = "tdt-0.6b-v2-q8_0";
const CPU_ENGINE: &str = "Parakeet";
const CPU_MODEL: &str = "parakeet-unified-en-0.6b";

#[derive(Debug, Serialize)]
pub struct SetupOption {
    pub engine: String,
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

/// Builds the recommendation for `devices`. The GPU option appears only when
/// parakeet.cpp has a GPU build for this platform and a suitable card exists.
pub fn recommend(devices: &[GraphicsDevice]) -> SetupRecommendation {
    let graphics_card = if crate::parakeet::gpu_backend_available() {
        suitable_graphics_card(devices).map(|device| device.description.clone())
    } else {
        None
    };

    let mut options = Vec::new();
    if let Some(card) = &graphics_card {
        options.extend(option(
            GPU_ENGINE,
            GPU_MODEL,
            "Best quality, uses your graphics card",
            &format!("Fast, accurate English dictation on your {}.", card),
        ));
    }
    options.extend(option(
        CPU_ENGINE,
        CPU_MODEL,
        "Works on any computer",
        "Accurate English dictation on your processor. A little slower.",
    ));

    crate::log_info!(
        "Setup recommendation: graphics card {:?}, first choice {:?}",
        graphics_card,
        options.first().map(|choice| &choice.model_size)
    );
    SetupRecommendation {
        graphics_card,
        options,
    }
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
fn option(engine: &str, model_size: &str, title: &str, description: &str) -> Option<SetupOption> {
    let Some(model) = ModelManager::find_model(engine, model_size) else {
        crate::log_warn!(
            "Setup recommendation names {} / {}, which is not in the model catalog",
            engine,
            model_size
        );
        return None;
    };
    Some(SetupOption {
        engine: engine.to_string(),
        model_size: model_size.to_string(),
        title: title.to_string(),
        description: description.to_string(),
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
            total_memory_bytes: gigabytes * 1024 * 1024 * 1024,
        }
    }

    #[test]
    fn recommended_models_exist_in_the_catalog() {
        assert!(ModelManager::find_model(GPU_ENGINE, GPU_MODEL).is_some());
        assert!(ModelManager::find_model(CPU_ENGINE, CPU_MODEL).is_some());
    }

    #[test]
    fn picks_the_discrete_card_with_the_most_memory() {
        let devices = [
            device(GraphicsKind::Discrete, 8),
            device(GraphicsKind::Discrete, 20),
            device(GraphicsKind::Integrated, 32),
        ];
        let card = suitable_graphics_card(&devices).unwrap();
        assert_eq!(card.total_memory_bytes, 20 * 1024 * 1024 * 1024);
    }

    #[test]
    fn small_cards_are_not_suitable() {
        let devices = [device(GraphicsKind::Discrete, 2)];
        assert!(suitable_graphics_card(&devices).is_none());
    }

    #[test]
    fn processor_option_is_always_offered() {
        let recommendation = recommend(&[]);
        assert!(recommendation.graphics_card.is_none());
        assert_eq!(recommendation.options.len(), 1);
        assert_eq!(recommendation.options[0].engine, CPU_ENGINE);
    }
}
