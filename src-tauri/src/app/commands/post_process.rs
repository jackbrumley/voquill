use crate::post_process::provider_api;

#[tauri::command]
pub async fn test_cleanup_api(
    api_key: String,
    api_url: String,
    model: String,
) -> Result<String, String> {
    provider_api::test_connection(&api_key, &api_url, &model).await
}
