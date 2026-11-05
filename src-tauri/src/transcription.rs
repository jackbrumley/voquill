use async_trait::async_trait;
use reqwest::multipart;
use serde_json::Value;

#[derive(Debug)]
pub enum TranscriptionError {
    Network(String),
    Model(String),
    Audio(String),
}

impl std::fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::Model(e) => write!(f, "Model error: {}", e),
            Self::Audio(e) => write!(f, "Audio error: {}", e),
        }
    }
}

impl std::error::Error for TranscriptionError {}

#[async_trait]
pub trait TranscriptionService {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError>;
    fn service_name(&self) -> &'static str;
}

/// Longest piece of audio uploaded in one API request. OpenAI-compatible
/// endpoints reject uploads over 25 MB; 600 s of 16 kHz mono 16-bit WAV is
/// about 19 MB, so longer recordings are split at pauses into pieces of at
/// most this length.
const API_MAX_PIECE_SECONDS: f32 = 600.0;

pub struct APITranscriptionService {
    pub api_key: String,
    pub api_url: String,
    pub api_model: String,
}

#[async_trait]
impl TranscriptionService for APITranscriptionService {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        if self.api_model.is_empty() {
            return Err(TranscriptionError::Model(
                "API model is not configured. Set a model in Settings.".into(),
            ));
        }

        let pieces = split_wav_for_upload(audio_data)?;
        if pieces.len() > 1 {
            crate::log_info!(
                "API transcription: uploading long audio in {} pieces of at most {}s",
                pieces.len(),
                API_MAX_PIECE_SECONDS
            );
        }

        let mut piece_texts = Vec::with_capacity(pieces.len());
        for piece in pieces {
            let piece_text = self.request_transcription(piece, language, prompt).await?;
            let piece_text = piece_text.trim();
            if !piece_text.is_empty() {
                piece_texts.push(piece_text.to_string());
            }
        }
        Ok(piece_texts.join(" "))
    }

    fn service_name(&self) -> &'static str {
        "API Transcription"
    }
}

impl APITranscriptionService {
    /// Uploads one WAV (within the API's size limit) and returns its text.
    async fn request_transcription(
        &self,
        wav: Vec<u8>,
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let client = reqwest::Client::new();

        let mut form = multipart::Form::new()
            .part(
                "file",
                multipart::Part::bytes(wav)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| TranscriptionError::Audio(e.to_string()))?,
            )
            .text("model", self.api_model.clone());

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        if let Some(p) = prompt {
            form = form.text("prompt", p.to_string());
        }

        let response = client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| TranscriptionError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .map_err(|e| TranscriptionError::Network(e.to_string()))?;
            return Err(TranscriptionError::Network(format!(
                "API error: {}",
                error_text
            )));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| TranscriptionError::Network(e.to_string()))?;

        let text = json["text"].as_str().unwrap_or("").to_string();

        Ok(text)
    }
}

/// Splits a 16 kHz mono 16-bit WAV into WAVs short enough to upload. Audio
/// that already fits is passed through untouched.
fn split_wav_for_upload(wav: &[u8]) -> Result<Vec<Vec<u8>>, TranscriptionError> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(wav))
        .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
    let spec = reader.spec();
    let duration_seconds = reader.duration() as f32 / spec.sample_rate as f32;
    if duration_seconds <= API_MAX_PIECE_SECONDS {
        return Ok(vec![wav.to_vec()]);
    }
    if spec.channels != 1 || spec.sample_rate != 16000 || spec.bits_per_sample != 16 {
        return Err(TranscriptionError::Audio(format!(
            "Cannot split {}ch {}Hz {}-bit audio for upload; expected 16 kHz mono 16-bit WAV",
            spec.channels, spec.sample_rate, spec.bits_per_sample
        )));
    }

    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
    crate::audio::chunking::split_at_pauses(&samples, spec.sample_rate, API_MAX_PIECE_SECONDS)
        .into_iter()
        .map(|piece| {
            crate::audio::conversion::write_whisper_wav(&samples[piece])
                .map_err(|e| TranscriptionError::Audio(e.to_string()))
        })
        .collect()
}

pub async fn test_api_key(
    api_key: &str,
    api_url: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    // Create a minimal test audio file (silence)
    let test_audio = create_test_audio();

    let form = multipart::Form::new()
        .part(
            "file",
            multipart::Part::bytes(test_audio)
                .file_name("test.wav")
                .mime_str("audio/wav")?,
        )
        .text("model", "gpt-transcribe");

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    Ok(response.status().is_success())
}

fn create_test_audio() -> Vec<u8> {
    // Create a minimal WAV file with 1 second of silence
    let sample_rate = 16000u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let duration_samples = sample_rate; // 1 second

    let mut wav_data = Vec::new();

    // WAV header
    wav_data.extend_from_slice(b"RIFF");
    let file_size = 36 + duration_samples * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&(file_size - 8).to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");

    // Format chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // Audio format (PCM)
    wav_data.extend_from_slice(&channels.to_le_bytes());
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());

    // Data chunk
    wav_data.extend_from_slice(b"data");
    let data_size = duration_samples * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&data_size.to_le_bytes());

    // Silent audio data (zeros)
    for _ in 0..duration_samples {
        wav_data.extend_from_slice(&0i16.to_le_bytes());
    }

    wav_data
}
