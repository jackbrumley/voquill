//! The catalog of downloadable models: which engines exist and which models
//! each one offers. Storage, download and on-disk layout live in the parent
//! module.

use super::{ModelInfo, ModelManager};
use crate::engines::{EngineInfo, ModelEngine};

/// One whisper.cpp model definition. Each spec is expanded into a CPU and a
/// GPU engine variant so the two catalogs can never drift apart.
struct WhisperModelSpec {
    size: &'static str,
    label: &'static str,
    file_size: u64,
    download_url: &'static str,
    sha256: &'static str,
    cpu_description: &'static str,
    gpu_description: &'static str,
    recommended: bool,
}

/// One post-process (GGUF) model definition. Same deal: expanded into CPU
/// ("Post-Process (Local)") and GPU ("Post-Process (GPU)") engine variants
/// sharing the same model file.
struct PostProcessModelSpec {
    size: &'static str,
    label: &'static str,
    file_size: u64,
    download_url: &'static str,
    cpu_description: &'static str,
    gpu_description: &'static str,
    recommended: bool,
}

impl ModelManager {
    /// Looks up a single model by engine name + model size.
    pub fn find_model(engine: ModelEngine, size: &str) -> Option<ModelInfo> {
        Self::get_available_models()
            .into_iter()
            .find(|m| m.engine == engine && m.size == size)
    }

    pub fn get_available_models() -> Vec<ModelInfo> {
        let cpu_models = Self::cpu_models();
        let gpu_models = Self::gpu_models();
        let mut all = Vec::with_capacity(cpu_models.len() + gpu_models.len());
        all.extend(cpu_models);
        all.extend(gpu_models);
        all.extend(Self::parakeet_models());
        all.extend(Self::parakeet_gpu_models());
        all.extend(Self::post_process_models());
        all
    }

    fn whisper_model_specs() -> Vec<WhisperModelSpec> {
        vec![
            WhisperModelSpec {
                size: "tiny.en", label: "Tiny (English)", file_size: 77_600_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
                sha256: "be07098a4cc50130a511ca096303ad371c513297a7d4a093047d9ca4378f8776",
                cpu_description: "Lightning fast, best for simple commands.",
                gpu_description: "Lightning fast with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "distil-small.en", label: "Distil-Small (English)", file_size: 175_000_000,
                download_url: "https://huggingface.co/distil-whisper/distil-small.en/resolve/main/ggml-distil-small.en.bin",
                sha256: "e8a676964fd3f78b021a385f078a18863712ca10fdc907a685eee9c0e71d7a62",
                cpu_description: "Perfect balance of speed and high accuracy.",
                gpu_description: "Fast and accurate with GPU acceleration. Requires a compatible GPU.",
                recommended: true,
            },
            WhisperModelSpec {
                size: "base.en", label: "Base (English)", file_size: 147_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
                sha256: "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d",
                cpu_description: "Standard choice for general dictation.",
                gpu_description: "Standard choice with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "small.en", label: "Small (English)", file_size: 483_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                cpu_description: "Great accuracy for complex vocabulary.",
                gpu_description: "Great accuracy with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "medium.en", label: "Medium (English)", file_size: 1_500_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                cpu_description: "High accuracy. Needs a powerful computer or GPU.",
                gpu_description: "High accuracy with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "large-v3-turbo-q8_0", label: "Large Turbo", file_size: 874_188_075,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin",
                sha256: "317eb69c11673c9de1e1f0d459b253999804ec71ac4c23c17ecf5fbe24e259a1",
                cpu_description: "Highest accuracy, many languages. Slow without a GPU.",
                gpu_description: "Highest accuracy, many languages, with GPU acceleration. Best choice for a capable GPU.",
                recommended: false,
            },
        ]
    }

    fn whisper_models(engine: ModelEngine) -> Vec<ModelInfo> {
        Self::whisper_model_specs()
            .into_iter()
            .map(|spec| {
                Self::model_info(
                    engine,
                    spec.size,
                    spec.label,
                    spec.file_size,
                    spec.download_url,
                    spec.sha256,
                    if engine.uses_gpu() {
                        spec.gpu_description
                    } else {
                        spec.cpu_description
                    },
                    spec.recommended,
                )
            })
            .collect()
    }

    fn cpu_models() -> Vec<ModelInfo> {
        Self::whisper_models(ModelEngine::WhisperCpu)
    }

    fn gpu_models() -> Vec<ModelInfo> {
        Self::whisper_models(ModelEngine::WhisperGpu)
    }

    fn parakeet_models() -> Vec<ModelInfo> {
        vec![
            Self::model_info(ModelEngine::ParakeetCpu, "parakeet-tdt-0.6b-v3", "Parakeet TDT 0.6B (Multilingual)", 680_000_000,
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2",
                "",
                "NVIDIA Parakeet model, 25 languages. Requires sherpa-onnx sidecar. Fast on CPU.", true),
            Self::model_info(ModelEngine::ParakeetCpu, "parakeet-unified-en-0.6b", "Parakeet Unified EN 0.6B (English)", 631_000_000,
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-non-streaming.tar.bz2",
                "",
                "NVIDIA Parakeet English-only model. Requires sherpa-onnx sidecar. Fast on CPU.", false),
        ]
    }

    /// parakeet.cpp GGUF models for the "Parakeet (GPU)" engine (Vulkan on
    /// Windows/Linux, Metal on macOS). Published by the parakeet.cpp author;
    /// NVIDIA's models are CC-BY-4.0.
    fn parakeet_gpu_models() -> Vec<ModelInfo> {
        vec![
            Self::model_info(ModelEngine::ParakeetGpu, "tdt-0.6b-v2-q8_0", "Parakeet TDT 0.6B v2 (English)", 903_835_936,
                "https://huggingface.co/mudler/parakeet-cpp-gguf/resolve/main/tdt-0.6b-v2-q8_0.gguf",
                "2027e2e1a4dc60ccdd8558f93b15e7c0db4ef8895b4e82e889f3a6275d8119c6",
                "NVIDIA Parakeet English model on your graphics card. Very accurate and very fast.", true),
            Self::model_info(ModelEngine::ParakeetGpu, "tdt-0.6b-v3-q8_0", "Parakeet TDT 0.6B v3 (Multilingual)", 940_663_680,
                "https://huggingface.co/mudler/parakeet-cpp-gguf/resolve/main/tdt-0.6b-v3-q8_0.gguf",
                "4d69a4a6683f4f2d952bad794c1357ca6eb628027695b4699c5a9ad4cd07d757",
                "NVIDIA Parakeet model, 25 languages, on your graphics card. Very fast.", false),
        ]
    }

    fn post_process_model_specs() -> Vec<PostProcessModelSpec> {
        vec![
            PostProcessModelSpec {
                size: "qwen2.5-1.5b-instruct", label: "Qwen 2.5 1.5B", file_size: 700_000_000,
                download_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
                cpu_description: "Small local model for post-processing. Fixes punctuation, capitalization, and removes filler words. ~3-5s on CPU.",
                gpu_description: "Small local model for post-processing, GPU-accelerated via Vulkan. Fixes punctuation, capitalization, and removes filler words.",
                recommended: true,
            },
            PostProcessModelSpec {
                size: "llama-3.2-1b-instruct", label: "Llama 3.2 1B", file_size: 650_000_000,
                download_url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
                cpu_description: "Meta's lightweight instruct model. Good for post-processing on modest hardware.",
                gpu_description: "Meta's lightweight instruct model with GPU acceleration via Vulkan.",
                recommended: false,
            },
        ]
    }

    fn post_process_models() -> Vec<ModelInfo> {
        [ModelEngine::PostProcessCpu, ModelEngine::PostProcessGpu]
            .into_iter()
            .flat_map(|engine| {
                let use_gpu = engine.uses_gpu();
                Self::post_process_model_specs()
                    .into_iter()
                    .map(move |spec| {
                        Self::model_info(
                            engine,
                            spec.size,
                            spec.label,
                            spec.file_size,
                            spec.download_url,
                            "",
                            if use_gpu {
                                spec.gpu_description
                            } else {
                                spec.cpu_description
                            },
                            spec.recommended,
                        )
                    })
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn model_info(
        engine: ModelEngine,
        size: &str,
        label: &str,
        file_size: u64,
        download_url: &str,
        sha256: &str,
        description: &str,
        recommended: bool,
    ) -> ModelInfo {
        ModelInfo {
            engine,
            size: size.to_string(),
            label: label.to_string(),
            file_size,
            download_url: download_url.to_string(),
            sha256: sha256.to_string(),
            description: description.to_string(),
            recommended,
            category: engine.category(),
        }
    }

    /// Every engine (transcription and post-processing) that has at least
    /// one model.
    pub fn get_available_engines() -> Vec<EngineInfo> {
        let models = Self::get_available_models();
        ModelEngine::ALL
            .into_iter()
            .filter(|engine| models.iter().any(|model| model.engine == *engine))
            .map(EngineInfo::from)
            .collect()
    }
}
