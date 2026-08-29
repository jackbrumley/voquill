use std::collections::HashSet;

use crate::{
    audio,
    diarization::{DiarizationResult, Segment},
    history,
};
use tauri::{Emitter, Manager};

fn wav_duration_secs(wav_data: &[u8]) -> Result<f64, String> {
    let reader =
        hound::WavReader::new(std::io::Cursor::new(wav_data)).map_err(|e| e.to_string())?;
    Ok(reader.duration() as f64 / reader.spec().sample_rate as f64)
}

#[tauri::command]
pub async fn transcribe_audio_file(
    path: String,
    app_handle: tauri::AppHandle,
) -> Result<DiarizationResult, String> {
    let session_uuid = uuid::Uuid::new_v4().to_string();
    crate::log_info!(
        "[session:{}] transcribe_audio_file: {}",
        &session_uuid[..8],
        path
    );

    let app_state = app_handle.state::<crate::AppState>();
    let current_config = {
        let guard = app_state.config.lock().unwrap();
        guard.clone()
    };

    match transcribe_audio_file_inner(&path, &session_uuid, &app_handle, &current_config).await {
        Ok(result) => Ok(result),
        Err(error) => {
            crate::log_info!(
                "[session:{}] File transcription failed: {}",
                &session_uuid[..8],
                error
            );
            let _ = history::add_history_item(&history::NewHistoryItem {
                session_uuid: &session_uuid,
                status: "failed",
                text: "",
                raw_text: None,
                error_message: Some(&error),
                segments: None,
                audio_file: None,
                duration_secs: None,
                engine: Some(&current_config.local_engine),
                source: Some("file"),
                language: Some(&current_config.language),
                prompt_name: current_config.resolve_post_process_prompt_name().as_deref(),
                limit: Some(current_config.history_limit),
            });
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("history-updated", ());
            }
            Err(error)
        }
    }
}

async fn transcribe_audio_file_inner(
    path: &str,
    session_uuid: &str,
    app_handle: &tauri::AppHandle,
    current_config: &crate::config::Config,
) -> Result<DiarizationResult, String> {
    let audio_data = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    crate::log_info!("File size: {} bytes", audio_data.len());

    let wav_data = audio::convert_audio_file_for_whisper(&audio_data)
        .map_err(|e| format!("Failed to convert audio: {}", e))?;
    let duration_secs = wav_duration_secs(&wav_data).ok();

    let app_state = app_handle.state::<crate::AppState>();
    let engine_factory_state = app_state.engine_factory.clone();
    let service = engine_factory_state
        .create_service(current_config)
        .await
        .map_err(|e| format!("Failed to create transcription service: {}", e))?;
    let service_name = service.service_name().to_string();

    let language = current_config.language.clone();
    let prompt_hint = current_config.resolve_prompt_hint();
    let prompt_name = current_config.resolve_post_process_prompt_name();
    let lang_code = match language.as_str() {
        "auto" => None,
        "en-AU" => Some("en"),
        "en-GB" => Some("en"),
        "en-US" => Some("en"),
        code => Some(code),
    };

    crate::log_info!(
        "Transcription params: lang={:?}, lang_code={:?}, dictionary={:?}, prompt_hint={:?}",
        language,
        lang_code,
        current_config.dictionary,
        prompt_hint,
    );

    let saved_audio_file = if current_config.enable_recording_logs {
        match crate::paths::debug_recordings_dir() {
            Ok(dir) => {
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let file_name = format!("import_{}_{}.wav", timestamp, &session_uuid[..8]);
                let file_path = dir.join(&file_name);
                if let Err(e) = std::fs::write(&file_path, &wav_data) {
                    crate::log_warn!("Failed to save debug import audio: {}", e);
                    None
                } else {
                    crate::log_info!("Debug import audio saved: {:?}", file_path);
                    Some(file_name)
                }
            }
            Err(e) => {
                crate::log_warn!("Failed to get debug recordings directory: {}", e);
                None
            }
        }
    } else {
        None
    };

    // ── Diarization temp file ──
    // The Python runner reads audio via soundfile/libsnfile, which cannot decode
    // compressed containers (m4a/aac). Write the decoded WAV to a temp file so
    // diarization always receives a format libsndfile supports.
    let diar_path: Option<std::path::PathBuf> = if current_config.diarization_enabled_files {
        let temp_dir = crate::paths::temp_dir();
        let _ = std::fs::create_dir_all(&temp_dir);
        let temp_path = temp_dir.join(format!(
            "file_{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        match std::fs::write(&temp_path, &wav_data) {
            Ok(()) => Some(temp_path),
            Err(e) => {
                crate::log_warn!("Failed to write temp audio for diarization: {}", e);
                None
            }
        }
    } else {
        None
    };

    // ── Transcription: per-segment or full-file ──
    let diarization_cluster_threshold = current_config.diarization_cluster_threshold;
    let result = match &diar_path {
        None => {
            let text = service
                .transcribe(&wav_data, lang_code, prompt_hint.as_deref())
                .await
                .map_err(|e| format!("Transcription failed: {}", e))?;
            crate::log_info!(
                "Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            DiarizationResult {
                text,
                segments: vec![],
                provider: "none".to_string(),
            }
        }
        Some(diar_path) => match run_diarization(
            app_handle,
            &diar_path.to_string_lossy(),
            diarization_cluster_threshold,
        )
        .await
        {
            Ok(mut diar) if !diar.segments.is_empty() => {
                crate::log_info!(
                    "Diarization returned {} segments from {}",
                    diar.segments.len(),
                    diar.provider
                );

                let full_duration = wav_duration_secs(&wav_data)?;
                let mut segment_texts: Vec<(Option<String>, String)> = Vec::new();

                for seg in &diar.segments {
                    let start = seg.start_sec.unwrap_or(0.0);
                    let end = seg.end_sec.unwrap_or(full_duration);

                    let seg_wav = audio::extract_segment_wav(&wav_data, start, end)
                        .map_err(|e| format!("Failed to extract segment: {}", e))?;

                    let seg_text = service
                        .transcribe(&seg_wav, lang_code, prompt_hint.as_deref())
                        .await
                        .map_err(|e| format!("Segment transcription failed: {}", e))?;
                    crate::log_info!(
                        "Segment [{}]: \"{}\"",
                        seg.speaker.as_deref().unwrap_or("?"),
                        seg_text
                    );
                    segment_texts.push((seg.speaker.clone(), seg_text));
                }

                let unique_speakers: HashSet<Option<&str>> =
                    segment_texts.iter().map(|(s, _)| s.as_deref()).collect();

                if unique_speakers.len() <= 1 {
                    let full_text: String = segment_texts
                        .iter()
                        .map(|(_, t)| t.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    diar.text = full_text;
                    diar.segments.clear();
                } else {
                    let labeled: String = segment_texts
                        .iter()
                        .map(|(s, t)| format!("[{}] {}", s.as_deref().unwrap_or("Speaker"), t))
                        .collect::<Vec<_>>()
                        .join("\n");
                    diar.text = labeled;
                    diar.segments = segment_texts
                        .into_iter()
                        .map(|(speaker, text)| Segment {
                            speaker,
                            text,
                            start_sec: None,
                            end_sec: None,
                        })
                        .collect();
                }

                diar
            }
            Ok(mut diar) => {
                crate::log_warn!("Diarization returned 0 segments — transcribing full file");
                let text = service
                    .transcribe(&wav_data, lang_code, prompt_hint.as_deref())
                    .await
                    .map_err(|e| format!("Transcription failed: {}", e))?;
                crate::log_info!(
                    "Transcription received ({}): \"{}\"",
                    service.service_name(),
                    text
                );
                diar.text = text;
                diar
            }
            Err(e) => {
                crate::log_warn!("Diarization failed, transcribing full file: {}", e);
                let text = service
                    .transcribe(&wav_data, lang_code, prompt_hint.as_deref())
                    .await
                    .map_err(|e| format!("Transcription failed: {}", e))?;
                crate::log_info!(
                    "Transcription received ({}): \"{}\"",
                    service.service_name(),
                    text
                );
                DiarizationResult {
                    text,
                    segments: vec![],
                    provider: "none".to_string(),
                }
            }
        },
    };

    // ── Cleanup temp file ──
    if let Some(path) = &diar_path {
        let _ = std::fs::remove_file(path);
    }

    // ── Filler word removal (pre-processing, no LLM needed) ──
    let cleaned_text = crate::text_cleanup::clean_transcription(
        &result.text,
        current_config.filler_word_removal_enabled,
        &current_config.custom_filler_words,
    );

    // Save the raw (pre-post-process) text for history display
    let file_raw_text = if current_config.post_process_enabled {
        Some(cleaned_text.clone())
    } else {
        None
    };

    // ── Post-processing ──
    let result_text = if !cleaned_text.trim().is_empty() && current_config.post_process_enabled {
        crate::log_info!("Post-processing file transcription...");
        let post_process_factory = app_state.post_process_factory.clone();
        match post_process_factory.get_service(current_config).await {
            Ok(processor) => match processor
                .post_process(
                    &cleaned_text,
                    &current_config.resolve_post_process_prompt(),
                    &current_config.resolve_user_prompt_template(),
                    current_config.resolve_max_output_tokens(),
                )
                .await
            {
                Ok(cleaned) => {
                    crate::log_info!(
                        "Post-processed ({}): \"{}\"",
                        processor.service_name(),
                        cleaned
                    );
                    cleaned
                }
                Err(e) => {
                    crate::log_warn!("Post-processing failed, using cleaned text: {}", e);
                    if matches!(e, crate::post_process::PostProcessError::Network(_)) {
                        post_process_factory.invalidate_local();
                    }
                    cleaned_text.clone()
                }
            },
            Err(e) => {
                crate::log_warn!(
                    "Could not create post-process service, using cleaned text: {}",
                    e
                );
                cleaned_text.clone()
            }
        }
    } else {
        cleaned_text.clone()
    };

    if result_text.trim().is_empty() {
        let _ = history::add_history_item(&history::NewHistoryItem {
            session_uuid,
            status: "empty",
            text: "",
            raw_text: file_raw_text.as_deref(),
            error_message: Some("Transcription was empty or contained no speech"),
            segments: None,
            audio_file: saved_audio_file.as_deref(),
            duration_secs,
            engine: Some(&service_name),
            source: Some("file"),
            language: Some(&language),
            prompt_name: prompt_name.as_deref(),
            limit: Some(current_config.history_limit),
        });
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }

        return Ok(DiarizationResult {
            text: String::new(),
            segments: vec![],
            provider: "none".to_string(),
        });
    }

    // ── Save to history ──
    let result = DiarizationResult {
        text: result_text,
        segments: result.segments,
        provider: result.provider,
    };
    let segments_json = if result.segments.is_empty() {
        None
    } else {
        serde_json::to_string(&result.segments).ok()
    };
    if let Err(e) = history::add_history_item(&history::NewHistoryItem {
        session_uuid,
        status: "success",
        text: &result.text,
        raw_text: file_raw_text.as_deref(),
        error_message: None,
        segments: segments_json.as_deref(),
        audio_file: saved_audio_file.as_deref(),
        duration_secs,
        engine: Some(&service_name),
        source: Some("file"),
        language: Some(&language),
        prompt_name: prompt_name.as_deref(),
        limit: Some(current_config.history_limit),
    }) {
        crate::log_warn!("Failed to save history: {}", e);
    }
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.emit("history-updated", ());
    }

    Ok(result)
}

/// Run diarization on a decoded WAV file via the Python runner.
async fn run_diarization(
    app_handle: &tauri::AppHandle,
    audio_path: &str,
    cluster_threshold: f32,
) -> Result<DiarizationResult, String> {
    let app_state = app_handle.state::<crate::AppState>();
    let runner = app_state
        .get_or_start_python_runner(app_handle)
        .await
        .map_err(|e| {
            crate::log_warn!("Failed to start Python runner for diarization: {}", e);
            e
        })?;

    runner.diarize(audio_path, cluster_threshold).await
}
