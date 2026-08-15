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
    crate::log_info!("transcribe_audio_file: {}", path);

    let audio_data = std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    crate::log_info!("File size: {} bytes", audio_data.len());

    let wav_data = audio::convert_audio_file_for_whisper(&audio_data)
        .map_err(|e| format!("Failed to convert audio: {}", e))?;

    let app_state = app_handle.state::<crate::AppState>();
    let current_config = {
        let guard = app_state.config.lock().unwrap();
        guard.clone()
    };
    let engine_factory_state = app_state.engine_factory.clone();
    let service = engine_factory_state
        .create_service(&current_config)
        .await
        .map_err(|e| format!("Failed to create transcription service: {}", e))?;

    let language = current_config.language.clone();
    let dictionary_words = current_config.dictionary.clone();
    let lang_code = match language.as_str() {
        "auto" => None,
        "en-AU" => Some("en"),
        "en-GB" => Some("en"),
        "en-US" => Some("en"),
        code => Some(code),
    };

    let mut prompt_hint: Option<String> = match language.as_str() {
        "en-AU" => Some("Australian spelling.".to_string()),
        "en-GB" => Some("British spelling.".to_string()),
        "en-US" => Some("American spelling.".to_string()),
        _ => None,
    };

    if !dictionary_words.is_empty() {
        let dict_str = dictionary_words.join(", ");
        prompt_hint = match prompt_hint {
            Some(hint) => Some(format!("{}, {}", hint, dict_str)),
            None => Some(dict_str),
        };
    }

    crate::log_info!(
        "Transcription params: lang={:?}, lang_code={:?}, dictionary={:?}, prompt_hint={:?}",
        language,
        lang_code,
        dictionary_words,
        prompt_hint,
    );

    // ── Diarization temp file ──
    // The Python runner reads audio via soundfile/libsnfile, which cannot decode
    // compressed containers (m4a/aac). Write the decoded WAV to a temp file so
    // diarization always receives a format libsndfile supports.
    let diar_path: Option<std::path::PathBuf> = if current_config.diarization_enabled_files {
        let temp_dir = std::env::temp_dir().join("foss-voquill");
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
            &app_handle,
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

    // ── Post-processing ──
    let result_text = if !result.text.trim().is_empty() && current_config.post_process_enabled {
        crate::log_info!("Post-processing file transcription...");
        let post_process_factory = app_state.post_process_factory.clone();
        match post_process_factory.get_service(&current_config).await {
            Ok(processor) => match processor
                .post_process(&result.text, &current_config.post_process_prompt)
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
                    crate::log_warn!("Post-processing failed, using raw text: {}", e);
                    if matches!(e, crate::post_process::PostProcessError::Network(_)) {
                        post_process_factory.invalidate_local();
                    }
                    result.text.clone()
                }
            },
            Err(e) => {
                crate::log_warn!(
                    "Could not create post-process service, using raw text: {}",
                    e
                );
                result.text.clone()
            }
        }
    } else {
        result.text.clone()
    };

    if result_text.trim().is_empty() {
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
    if let Err(e) = history::add_history_item(&result.text, segments_json.as_deref()) {
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

    // Check if runner is already running (brief lock, no await)
    let needs_start = {
        let guard = app_state.python_runner.lock().unwrap();
        guard.is_none()
    };

    if needs_start {
        crate::log_info!("Lazily starting Python runner for diarization...");
        match crate::python_runner::PythonRunner::start(app_handle).await {
            Ok(runner) => {
                let mut guard = app_state.python_runner.lock().unwrap();
                // Only set if still None (another thread may have started it)
                if guard.is_none() {
                    *guard = Some(runner);
                }
            }
            Err(e) => {
                crate::log_warn!("Failed to start Python runner: {}", e);
                return Err(e);
            }
        }
    }

    // Clone the runner out of the state (brief lock)
    let runner = {
        let guard = app_state.python_runner.lock().unwrap();
        guard.clone()
    }
    .ok_or("Python runner not available after start attempt")?;

    runner.diarize(audio_path, cluster_threshold).await
}
