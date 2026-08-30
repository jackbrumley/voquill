use crate::diarization::DiarizationResult;
use tauri::Manager;

pub fn validate_audio_duration(
    audio_data: &[u8],
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    if audio_data.len() < 44 {
        return Err("Audio file too small".into());
    }
    let sample_rate = u32::from_le_bytes([
        audio_data[24],
        audio_data[25],
        audio_data[26],
        audio_data[27],
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);

    let mut data_size = 0u32;
    let mut pos = 36;
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[pos + 4],
            audio_data[pos + 5],
            audio_data[pos + 6],
            audio_data[pos + 7],
        ]);
        if chunk_id == b"data" {
            data_size = chunk_size;
            break;
        }
        pos += 8 + chunk_size as usize;
        if chunk_size % 2 == 1 {
            pos += 1;
        }
    }

    if data_size == 0 {
        return Err("No data chunk".into());
    }
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;

    crate::log_info!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 {
        return Err("Audio too short".into());
    }
    Ok(duration_seconds)
}

/// Run diarization on a recorded audio file via the Python runner.
pub async fn run_diarization_for_recording(
    app_handle: &tauri::AppHandle,
    audio_path: &std::path::Path,
    cluster_threshold: f32,
) -> Result<DiarizationResult, String> {
    if !crate::python_runner::is_diarization_ready() {
        crate::python_runner::download_diarization_models(app_handle).await?;
    }

    let app_state = app_handle.state::<crate::AppState>();
    let runner = app_state
        .get_or_start_python_runner(app_handle)
        .await
        .map_err(|e| {
            crate::log_warn!("Failed to start Python runner for diarization: {}", e);
            e
        })?;

    let path_str = audio_path.to_string_lossy().to_string();
    runner.diarize(&path_str, cluster_threshold).await
}

/// Run noise reduction on captured audio via the Python runner.
pub async fn run_noise_reduction(
    app_handle: &tauri::AppHandle,
    audio_data: &[u8],
    noise_reduction_strength: f32,
) -> Result<Vec<u8>, String> {
    let app_state = app_handle.state::<crate::AppState>();
    let runner = app_state
        .get_or_start_python_runner(app_handle)
        .await
        .map_err(|e| {
            crate::log_warn!("Failed to start Python runner for noise reduction: {}", e);
            e
        })?;

    let temp_dir = crate::paths::temp_dir();
    let _ = std::fs::create_dir_all(&temp_dir);
    let input_path = temp_dir.join(format!(
        "noise_input_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    std::fs::write(&input_path, audio_data)
        .map_err(|e| format!("Failed to write temp audio for noise reduction: {}", e))?;

    let path_str = input_path.to_string_lossy().to_string();
    let result = runner.enhance(&path_str, noise_reduction_strength).await;

    // Clean up input file regardless of outcome
    let _ = std::fs::remove_file(&input_path);

    let enhanced_path = result?;
    let enhanced_data = std::fs::read(&enhanced_path)
        .map_err(|e| format!("Failed to read enhanced audio: {}", e))?;

    // Clean up enhanced file
    let _ = std::fs::remove_file(&enhanced_path);

    Ok(enhanced_data)
}
