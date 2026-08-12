use serde::{Deserialize, Serialize};

/// A single segment of transcribed speech attributed to one speaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub speaker: Option<String>,
    pub text: String,
    pub start_sec: Option<f64>,
    pub end_sec: Option<f64>,
}

/// Result of a diarization run. Also used as the file import return type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationResult {
    /// Flat text of the (possibly labeled) transcript.
    /// Defaults to empty when deserializing from the Python server
    /// (which only returns segments + provider).
    #[serde(default)]
    pub text: String,
    /// Per-speaker segments (empty when diarization is disabled).
    #[serde(default)]
    pub segments: Vec<Segment>,
    /// Name of the diarization provider, or "none".
    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_provider() -> String {
    "none".to_string()
}

/// Trait for diarization backends. The Python runner is the initial
/// implementation; a future Rust-native ONNX path would also implement this.
#[allow(dead_code)]
pub trait DiarizationService: Send + Sync {
    async fn diarize(&self, audio_path: &str) -> Result<DiarizationResult, String>;
    fn service_name(&self) -> &'static str;
}
