use std::time::Duration;

use crate::diarization::DiarizationResult;

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const HEAVY_HTTP_TIMEOUT: Duration = Duration::from_secs(300);

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(DEFAULT_HTTP_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn heavy_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(HEAVY_HTTP_TIMEOUT)
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

/// Mirrors `EnhanceResponse` in `python-runner/enhancement/schemas.py`.
/// The contract test below fails if the two sides drift apart.
#[derive(serde::Deserialize)]
struct EnhanceResponse {
    enhanced_path: String,
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

    let client = heavy_http_client();
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

    let client = heavy_http_client();
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

    Ok(raw.enhanced_path)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TtsSynthesizeResponse {
    pub output_path: String,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub provider: String,
}

/// Renders speech from a fully resolved request (`crate::tts`).
pub async fn render_tts(
    base_url: &str,
    request: &crate::tts::RenderRequest,
) -> Result<TtsSynthesizeResponse, String> {
    let url = format!("{}/tts/render", base_url);
    let response = heavy_http_client()
        .post(&url)
        .json(request)
        .send()
        .await
        .map_err(|e| format!("TTS render request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("TTS render returned {}: {}", status, text));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse TTS render response: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENHANCEMENT_SCHEMA: &str = include_str!("../../../python-runner/enhancement/schemas.py");

    #[test]
    fn enhance_response_parses_the_python_runner_payload() {
        let payload = r#"{"enhanced_path": "/tmp/a_enhanced.wav", "provider": "noisereduce"}"#;
        let parsed: EnhanceResponse = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.enhanced_path, "/tmp/a_enhanced.wav");
    }

    #[test]
    fn enhance_response_matches_the_python_schema() {
        let response_class = ENHANCEMENT_SCHEMA
            .split("class EnhanceResponse(BaseModel):")
            .nth(1)
            .expect("python-runner schema declares EnhanceResponse");
        assert!(
            response_class.contains("enhanced_path: str"),
            "python-runner EnhanceResponse no longer declares `enhanced_path`"
        );
    }

    #[test]
    fn enhance_request_matches_the_python_schema() {
        let request_class = ENHANCEMENT_SCHEMA
            .split("class EnhanceRequest(BaseModel):")
            .nth(1)
            .and_then(|rest| rest.split("class ").next())
            .expect("python-runner schema declares EnhanceRequest");
        let request = serde_json::to_value(EnhanceRequest {
            audio_path: "a.wav",
            noise_reduction_strength: 0.5,
        })
        .unwrap();
        for field in request.as_object().unwrap().keys() {
            assert!(
                request_class.contains(&format!("{field}:")),
                "python-runner EnhanceRequest has no `{field}` field"
            );
        }
    }
}
