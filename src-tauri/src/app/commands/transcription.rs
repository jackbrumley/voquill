use crate::engine_factory;
use crate::{history, model_manager, transcription, AppState};
use tauri::Emitter;

#[tauri::command]
pub async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_available_engines() -> Result<Vec<crate::engines::EngineInfo>, String> {
    Ok(model_manager::ModelManager::get_available_engines())
}

/// Every model in the catalog with whether it is downloaded, in one call.
#[tauri::command]
pub async fn get_available_models() -> Result<Vec<model_manager::AvailableModel>, String> {
    let manager = model_manager::ModelManager::new()?;
    Ok(model_manager::ModelManager::get_available_models()
        .into_iter()
        .map(|model| model_manager::AvailableModel {
            downloaded: manager.is_model_downloaded(&model),
            model,
        })
        .collect())
}

#[tauri::command]
pub fn get_engine_capabilities(engine_name: String) -> engine_factory::EngineCapabilities {
    engine_factory::EngineFactory::engine_capabilities(&engine_name)
}

#[tauri::command]
pub async fn download_model(
    model_size: String,
    engine_name: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let engine = crate::engines::ModelEngine::parse_required(&engine_name)?;
    let model = model_manager::ModelManager::find_model(engine, &model_size)
        .ok_or_else(|| format!("Model {} not found for engine {}", model_size, engine_name))?;
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;

    let progress_handle = app_handle.clone();
    manager
        .download_model(&model, move |progress: model_manager::DownloadProgress| {
            let _ = progress_handle.emit("model-download-progress", progress);
        })
        .await?;

    // A completed download may satisfy a warm-up that startup skipped
    // because the model was missing. Re-arm whichever engine this model
    // belongs to so the first dictation reuses a warm service.
    let config = state.config.lock().unwrap().clone();

    if config.post_process_enabled
        && config.post_process_provider == crate::config::post_process::PostProcessProvider::Local
        && config.post_process_engine == engine_name
        && config.post_process_model == model_size
    {
        crate::app::bootstrap::spawn_post_process_warmup(
            state.post_process_factory.clone(),
            &config,
            &app_handle,
        );
    }

    if config.transcription_mode == crate::config::TranscriptionMode::Local
        && config.local_engine == engine_name
        && config.local_model_size == model_size
    {
        crate::app::bootstrap::spawn_engine_preload(state.engine_factory.clone(), &config);
    }

    crate::app::readiness::publish(&app_handle).await;
    Ok(())
}

/// Loads the configured transcription model into the engine cache and awaits
/// completion. Unlike the fire-and-forget startup/download preloads, this lets
/// the frontend deterministically verify engine loading (e.g. during initial
/// setup): a GPU engine either loads on GPU or records the fallback reason,
/// which `get_gpu_status` then reports.
#[tauri::command]
pub async fn preload_transcription_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("Tauri Command: preload_transcription_engine invoked");
    let config = { state.config.lock().unwrap().clone() };
    state.engine_factory.preload(&config).await;
    Ok(())
}

#[tauri::command]
pub fn get_current_status() -> String {
    crate::app::status::get_current_status()
}

#[tauri::command]
pub async fn get_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<history::HistoryItem>, String> {
    let limit = state.config.lock().unwrap().history_limit;
    history::load_history(limit).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_history(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<history::HistoryItem>, String> {
    let limit = state.config.lock().unwrap().history_limit;
    history::search_history(&query, limit).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_history_item(id: u64) -> Result<(), String> {
    history::delete_history_item(id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn unload_model(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.engine_factory.unload_all();
    crate::log_info!("Transcription model unloaded");
    Ok(())
}

/// The speech engine choices for initial setup, best first, chosen from the
/// graphics hardware Voquill detects.
#[tauri::command]
pub async fn get_setup_recommendation(
    state: tauri::State<'_, AppState>,
) -> Result<crate::setup_recommendation::SetupRecommendation, String> {
    let language = state.config.lock().unwrap().language.clone();
    let hardware = tokio::task::spawn_blocking(crate::hardware_detection::detect_hardware)
        .await
        .map_err(|e| format!("Hardware detection failed: {}", e))?;
    Ok(crate::setup_recommendation::recommend(&hardware, &language))
}

/// Whether a user who already has a working setup should see setup again as
/// a review after an update: whenever the completed setup revision is
/// behind. Hardware detection runs only then. The revision is recorded by
/// `complete_setup_review`, never here, so an unanswered review reopens.
#[tauri::command]
pub async fn get_setup_review(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::setup_review::SetupReview, String> {
    let config = state.config.lock().unwrap().clone();
    if !crate::setup_review::revision_is_behind(&config) {
        return Ok(crate::setup_review::SetupReview::not_due());
    }
    let recommendation =
        match tokio::task::spawn_blocking(crate::hardware_detection::detect_hardware).await {
            Ok(hardware) => Some(crate::setup_recommendation::recommend(
                &hardware,
                &config.language,
            )),
            Err(error) => {
                crate::log_warn!(
                "Hardware detection failed; showing the setup review without a recommendation: {}",
                error
            );
                None
            }
        };
    let review = crate::setup_review::review(&config, recommendation);
    crate::log_info!(
        "Setup review due (revision {} behind {}): recommended={:?}, already_recommended={}",
        config.completed_setup_revision,
        crate::setup_review::SETUP_REVISION,
        review.recommended.as_ref().map(|option| format!(
            "{} {}",
            option.engine.name(),
            option.model_size
        )),
        review.already_recommended
    );
    // Shown even when Voquill started hidden (autostart after an update), so
    // nobody misses the review.
    crate::app::tray::show_main_window(&app_handle);
    Ok(review)
}

/// Records that the user finished (or skipped) the current setup revision.
#[tauri::command]
pub fn complete_setup_review(state: tauri::State<'_, AppState>) -> Result<(), String> {
    record_setup_revision(&state)
}

fn record_setup_revision(state: &AppState) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    if config.completed_setup_revision == crate::setup_review::SETUP_REVISION {
        return Ok(());
    }
    config.completed_setup_revision = crate::setup_review::SETUP_REVISION;
    crate::config::storage::save_config(&config)?;
    crate::log_info!(
        "Setup revision {} recorded as completed",
        crate::setup_review::SETUP_REVISION
    );
    Ok(())
}

/// Whether the local speech engine is unloaded, loading or loaded. Changes
/// are pushed with the `engine-residency-changed` event.
#[tauri::command]
pub fn get_engine_residency(
    state: tauri::State<'_, AppState>,
) -> crate::engine_residency::EngineResidency {
    state.engine_factory.residency()
}
