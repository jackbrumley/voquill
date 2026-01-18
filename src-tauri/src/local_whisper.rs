use std::path::PathBuf;
use async_trait::async_trait;
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use crate::transcription::{TranscriptionService, TranscriptionError};
use crate::model_manager::ModelManager;
use hound;

pub struct LocalWhisperService {
    model_path: PathBuf,
}

impl LocalWhisperService {
    pub fn new(model_size: &str) -> Result<Self, TranscriptionError> {
        let model_manager = ModelManager::new()
            .map_err(|e| TranscriptionError::ModelError(e))?;
        
        let model_path = model_manager.get_model_path(model_size);
        
        if !model_path.exists() {
            return Err(TranscriptionError::ModelError(
                format!("Model {} not found. Please download it in settings.", model_size)
            ));
        }
        
        Ok(Self {
            model_path,
        })
    }
}

#[async_trait]
impl TranscriptionService for LocalWhisperService {
    async fn transcribe(&self, audio_data: &[u8]) -> Result<String, TranscriptionError> {
        // Convert WAV bytes to f32 samples
        let mut reader = hound::WavReader::new(std::io::Cursor::new(audio_data))
            .map_err(|e| TranscriptionError::AudioError(e.to_string()))?;
        
        let spec = reader.spec();
        if spec.channels != 1 || spec.sample_rate != 16000 {
            return Err(TranscriptionError::AudioError(
                format!("Unsupported audio format: {} channels, {}Hz. Expected 1 channel, 16000Hz.", spec.channels, spec.sample_rate)
            ));
        }

        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TranscriptionError::AudioError(e.to_string()))?;

        // Load context (lazy loading)
        let ctx = WhisperContext::new_with_params(
            self.model_path.to_str().ok_or_else(|| TranscriptionError::ModelError("Invalid model path".to_string()))?,
            WhisperContextParameters::default(),
        ).map_err(|e| TranscriptionError::ModelError(e.to_string()))?;
        
        let mut state = ctx.create_state().map_err(|e| TranscriptionError::ModelError(e.to_string()))?;

        // Configure transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        
        // Run transcription
        state.full(params, &samples)
            .map_err(|e| TranscriptionError::ModelError(e.to_string()))?;
        
        // Extract text
        let num_segments = state.full_n_segments();
        
        let mut result = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(segment_text) = segment.to_str_lossy() {
                    result.push_str(&segment_text);
                }
            }
        }
        
        Ok(result.trim().to_string())
    }
    
    fn service_name(&self) -> &'static str {
        "Local Whisper"
    }
}
