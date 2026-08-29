use std::time::Duration;

use crate::diarization::DiarizationResult;

const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

#[derive(serde::Serialize)]
struct DiarizeRequest<'a> {
    audio_path: &'a str,
    cluster_threshold: f32,
}

#[derive(serde::Serialize)]
struct EnhanceRequest<'a> {
    audio_path: &'a str,
    noise_reduction_strength: f32,
}

#[derive(serde::Deserialize)]
struct EnhanceResponse {
    output_path: String,
}

#[allow(dead_code)]
pub async fn check_health(base_url: &str) -> Result<(), String> {
    let url = format!("{}/health", base_url);
    let client = http_client();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Health check failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Health check returned status: {}",
            response.status()
        ))
    }
}

#[allow(dead_code)]
pub async fn get_capabilities(base_url: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/capabilities", base_url);
    let client = http_client();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Capabilities check failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Capabilities check returned status: {}",
            response.status()
        ));
    }

    #[derive(serde::Deserialize)]
    struct CapResponse {
        capabilities: Vec<String>,
        #[allow(dead_code)]
        version: String,
    }

    let cap: CapResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse capabilities: {}", e))?;

    Ok(cap.capabilities)
}

pub async fn diarize(
    base_url: &str,
    audio_path: &str,
    cluster_threshold: f32,
) -> Result<DiarizationResult, String> {
    let url = format!("{}/diarize", base_url);
    let body = DiarizeRequest {
        audio_path,
        cluster_threshold,
    };

    let client = http_client();
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

    let raw: DiarizationResult = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse diarization response: {}", e))?;

    Ok(raw)
}

pub async fn enhance(
    base_url: &str,
    audio_path: &str,
    noise_reduction_strength: f32,
) -> Result<String, String> {
    let url = format!("{}/enhance", base_url);
    let body = EnhanceRequest {
        audio_path,
        noise_reduction_strength,
    };

    let client = http_client();
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

    let raw: EnhanceResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse enhancement response: {}", e))?;

    Ok(raw.output_path)
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
    #[serde(default)]
    pub default_effect: Option<String>,
    #[serde(default)]
    pub default_pitch: Option<f32>,
    #[serde(default)]
    pub default_speed: Option<f32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BaseVoiceModelInfo {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub is_multi_speaker: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TtsSynthesizeResponse {
    pub output_path: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub provider: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomTtsSynthesizeParams {
    pub text: String,
    pub model_key: String,
    #[serde(default)]
    pub speaker_id: i32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_noise_scale")]
    pub noise_scale: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub sub_bass: f32,
    #[serde(default)]
    pub comb_mix: f32,
    #[serde(default)]
    pub flanger_mix: f32,
    #[serde(default)]
    pub radio_bandpass: bool,
    #[serde(default = "default_radio_drive")]
    pub radio_drive: f32,
    #[serde(default)]
    pub rf_noise: f32,
    #[serde(default = "default_chime")]
    pub opening_chime: String,
    #[serde(default = "default_chime")]
    pub closing_chime: String,
    pub output_path: Option<String>,
}

fn default_speed() -> f32 {
    1.0
}
fn default_noise_scale() -> f32 {
    0.667
}
fn default_radio_drive() -> f32 {
    1.0
}
fn default_chime() -> String {
    "none".to_string()
}

pub async fn get_tts_models(base_url: &str) -> Result<Vec<BaseVoiceModelInfo>, String> {
    let url = format!("{}/tts/models", base_url);
    let client = http_client();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to query TTS models: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("TTS models query returned {}: {}", status, text));
    }

    let models: Vec<BaseVoiceModelInfo> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse TTS models response: {}", e))?;

    Ok(models)
}

pub async fn get_tts_voices(base_url: &str) -> Result<Vec<VoicePersonaInfo>, String> {
    let url = format!("{}/tts/voices", base_url);
    let client = http_client();
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

    let client = http_client();
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

pub async fn synthesize_custom_tts(
    base_url: &str,
    params: &CustomTtsSynthesizeParams,
) -> Result<TtsSynthesizeResponse, String> {
    let url = format!("{}/tts/synthesize_custom", base_url);
    let client = http_client();
    let response = client
        .post(&url)
        .json(params)
        .send()
        .await
        .map_err(|e| format!("Custom TTS synthesis request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "Custom TTS synthesis returned {}: {}",
            status, text
        ));
    }

    let raw: TtsSynthesizeResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse custom TTS response: {}", e))?;

    Ok(raw)
}
