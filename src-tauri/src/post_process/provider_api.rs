use async_trait::async_trait;
use serde_json::Value;

use super::{PostProcessError, PostProcessService};

pub struct APIPostProcessService {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub system_prompt: String,
}

#[async_trait]
impl PostProcessService for APIPostProcessService {
    async fn post_process(&self, text: &str) -> Result<String, PostProcessError> {
        let messages = super::prompt::build_post_process_messages(text, &self.system_prompt);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": super::prompt::max_output_tokens(text),
            "temperature": 0.0,
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| PostProcessError::Network(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(PostProcessError::Api(format!(
                "API returned {}: {}",
                status, body_text
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| PostProcessError::Network(format!("Failed to parse response: {}", e)))?;

        let cleaned = data["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| PostProcessError::Api("No content in response".to_string()))?
            .trim()
            .to_string();
        let cleaned = cleaned.replace('\n', " ");
        let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

        Ok(cleaned)
    }

    fn service_name(&self) -> &'static str {
        "Post-Process (API)"
    }
}

pub async fn test_connection(
    api_key: &str,
    api_url: &str,
    model: &str,
    system_prompt: &str,
) -> Result<String, String> {
    let messages =
        super::prompt::build_post_process_messages("This is a test um sentence", system_prompt);

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": 256,
        "temperature": 0.0,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        return Err(format!("API returned {}: {}", status, body_text));
    }

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let cleaned = data["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "No content in response".to_string())?
        .trim()
        .to_string();
    let cleaned = cleaned.replace('\n', " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    Ok(cleaned)
}
