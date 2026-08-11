use async_trait::async_trait;

#[derive(Debug)]
pub enum PostProcessError {
    Network(String),
    Api(String),
}

impl std::fmt::Display for PostProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::Api(e) => write!(f, "API error: {}", e),
        }
    }
}

impl std::error::Error for PostProcessError {}

#[async_trait]
pub trait PostProcessService: Send + Sync {
    /// Cleans `text` using `system_prompt`. The prompt is request-scoped:
    /// editing it never requires rebuilding the underlying service.
    async fn post_process(
        &self,
        text: &str,
        system_prompt: &str,
    ) -> Result<String, PostProcessError>;
    fn service_name(&self) -> &'static str;
}

pub mod factory;
pub mod prompt;
pub mod provider_api;
pub mod provider_local;
