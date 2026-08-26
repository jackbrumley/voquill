use crate::diarization::{DiarizationResult, Segment};

#[derive(serde::Deserialize)]
struct RawDiarizeResponse {
    segments: Vec<Segment>,
    provider: String,
}

#[derive(serde::Deserialize)]
struct RawEnhanceResponse {
    enhanced_path: String,
    #[allow(dead_code)]
    provider: String,
}

pub async fn diarize(
    base_url: &str,
    audio_path: &str,
    cluster_threshold: f32,
) -> Result<DiarizationResult, String> {
    let url = format!("{}/diarize", base_url);
    let body = serde_json::json!({
        "audio_path": audio_path,
        "cluster_threshold": cluster_threshold,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Diarization request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Diarization returned {}: {}", status, text));
    }

    let raw: RawDiarizeResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse diarization response: {}", e))?;

    let labeled = raw
        .segments
        .iter()
        .map(|s| {
            let label = s.speaker.as_deref().unwrap_or("Speaker");
            format!("[{}] {}", label, s.text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(DiarizationResult {
        text: labeled,
        segments: raw.segments,
        provider: raw.provider,
    })
}

pub async fn enhance(
    base_url: &str,
    audio_path: &str,
    noise_reduction_strength: f32,
) -> Result<String, String> {
    let url = format!("{}/enhance", base_url);
    let body = serde_json::json!({
        "audio_path": audio_path,
        "noise_reduction_strength": noise_reduction_strength,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Enhancement request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Enhancement returned {}: {}", status, text));
    }

    let raw: RawEnhanceResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse enhance response: {}", e))?;

    Ok(raw.enhanced_path)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VoicePersonaInfo {
    pub id: String,
    pub name: String,
    pub persona: String,
    pub category: String,
    pub engine: String,
    pub description: String,
    pub is_ready: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TtsSynthesizeResponse {
    pub output_path: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub provider: String,
}

pub async fn get_tts_voices(base_url: &str) -> Result<Vec<VoicePersonaInfo>, String> {
    let url = format!("{}/tts/voices", base_url);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to query TTS voices: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("TTS voices query returned {}: {}", status, text));
    }

    let voices: Vec<VoicePersonaInfo> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse TTS voices response: {}", e))?;

    Ok(voices)
}

pub async fn synthesize_tts(
    base_url: &str,
    text: &str,
    voice_id: &str,
    speed: f32,
    effect: Option<&str>,
    pitch: Option<f32>,
    output_path: Option<&str>,
) -> Result<TtsSynthesizeResponse, String> {
    let url = format!("{}/tts/synthesize", base_url);
    let body = serde_json::json!({
        "text": text,
        "voice_id": voice_id,
        "speed": speed,
        "effect": effect.unwrap_or("clean"),
        "pitch": pitch.unwrap_or(0.0),
        "output_path": output_path,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS synthesis request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("TTS synthesis returned {}: {}", status, text));
    }

    let raw: TtsSynthesizeResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse TTS response: {}", e))?;

    Ok(raw)
}
