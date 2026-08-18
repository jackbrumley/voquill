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
