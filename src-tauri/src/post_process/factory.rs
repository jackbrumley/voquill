use crate::config::Config;
use crate::post_process::provider_api::APIPostProcessService;
use crate::post_process::provider_local::SidecarPostProcess;
use crate::post_process::PostProcessService;

pub struct PostProcessFactory;

impl PostProcessFactory {
    pub async fn create_service(
        config: &Config,
    ) -> Result<Box<dyn PostProcessService + Send + Sync>, String> {
        match config.post_process_provider {
            crate::config::PostProcessProvider::Api => Ok(Box::new(APIPostProcessService {
                api_key: config.post_process_api_key.clone(),
                api_url: config.post_process_api_url.clone(),
                model: config.post_process_model.clone(),
            })),
            crate::config::PostProcessProvider::Local => {
                let service = SidecarPostProcess::new(&config.post_process_model)
                    .await
                    .map_err(|e| format!("Failed to start local post-process: {}", e))?;
                Ok(Box::new(service))
            }
        }
    }
}
