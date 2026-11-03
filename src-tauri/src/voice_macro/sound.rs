use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::audio::decode::decode_compressed_audio;
use crate::audio::{AudioClip, PlaybackSlot};
use crate::config::voice_macro::{MacroSoundMode, VoiceMacroCommand};
use crate::AppState;

const MACRO_TRIGGER_SOUND_BYTES: &[u8] = include_bytes!("../../sounds/macro_trigger.mp3");

static TRIGGER_SOUND: OnceLock<Option<AudioClip>> = OnceLock::new();

pub fn stop_macro_sound_playback(app_state: &AppState) {
    app_state.audio_player.stop(PlaybackSlot::MacroFeedback);
}

fn trigger_sound() -> Option<&'static AudioClip> {
    TRIGGER_SOUND
        .get_or_init(
            || match decode_compressed_audio(MACRO_TRIGGER_SOUND_BYTES) {
                Ok(decoded) => {
                    crate::log_info!(
                        "Voice Macro sound decoded successfully ({} samples at {}Hz)",
                        decoded.samples.len(),
                        decoded.sample_rate
                    );
                    Some(AudioClip::from_decoded(decoded))
                }
                Err(e) => {
                    crate::log_warn!("Failed to decode embedded Voice Macro sound: {}", e);
                    None
                }
            },
        )
        .as_ref()
}

pub fn macro_sound_path(macro_id: &str) -> Result<PathBuf, String> {
    let dir = crate::paths::macro_sounds_dir()?;
    Ok(dir.join(format!("{}.wav", macro_id)))
}

pub fn play_macro_trigger_sound(app_state: &AppState) {
    let Some(clip) = trigger_sound() else {
        return;
    };
    if let Err(error) = app_state.play_audio(PlaybackSlot::MacroFeedback, clip.clone(), || {}) {
        crate::log_warn!("Voice Macro trigger playback error: {}", error);
    }
}

pub fn play_macro_sound(
    app_state: &AppState,
    command: &VoiceMacroCommand,
    global_feedback_enabled: bool,
) {
    match command.sound_mode {
        MacroSoundMode::None => {
            // Explicitly muted
        }
        MacroSoundMode::Default => {
            if global_feedback_enabled {
                play_macro_trigger_sound(app_state);
            }
        }
        MacroSoundMode::Tts | MacroSoundMode::CustomFile | MacroSoundMode::MicRecording => {
            if let Ok(path) = macro_sound_path(&command.id) {
                if path.exists() {
                    let _ = play_macro_sound_file(app_state, &path);
                    return;
                }
            }
            // Fallback to default chirp if custom audio missing on disk
            if global_feedback_enabled {
                play_macro_trigger_sound(app_state);
            }
        }
    }
}

pub fn play_macro_sound_file(app_state: &AppState, file_path: &Path) -> Result<(), String> {
    crate::log_info!("Loading macro sound file from {}", file_path.display());
    let bytes =
        std::fs::read(file_path).map_err(|e| format!("Failed to read macro sound file: {}", e))?;

    let decoded = decode_compressed_audio(&bytes)
        .map_err(|e| format!("Failed to decode macro sound file: {}", e))?;
    let clip = AudioClip::from_decoded(decoded);
    crate::log_info!(
        "Playing macro sound ({} samples at {}Hz)",
        clip.samples.len(),
        clip.sample_rate
    );

    app_state
        .play_audio(PlaybackSlot::MacroFeedback, clip, || {})
        .map_err(|error| {
            let message = format!("Voice Macro playback error: {}", error);
            crate::log_warn!("{}", message);
            message
        })
}

pub fn import_macro_audio_file(macro_id: &str, source_path: &str) -> Result<String, String> {
    let src_bytes = std::fs::read(source_path)
        .map_err(|e| format!("Failed to read source audio file '{}': {}", source_path, e))?;

    let decoded = decode_compressed_audio(&src_bytes)
        .map_err(|e| format!("Failed to decode source audio file: {}", e))?;

    let mono_samples = if decoded.channels > 1 {
        let chans = decoded.channels;
        decoded
            .samples
            .chunks_exact(chans)
            .map(|frame| frame.iter().sum::<f32>() / chans as f32)
            .collect::<Vec<f32>>()
    } else {
        decoded.samples
    };

    let dest_path = macro_sound_path(macro_id)?;

    save_samples_to_wav(&dest_path, &mono_samples, decoded.sample_rate)?;
    crate::log_info!(
        "Imported custom macro audio for '{}' ({} samples at {}Hz) to {}",
        macro_id,
        mono_samples.len(),
        decoded.sample_rate,
        dest_path.display()
    );

    Ok(dest_path.to_string_lossy().to_string())
}

pub fn save_macro_mic_recording(
    macro_id: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<String, String> {
    if samples.is_empty() {
        return Err("Recorded audio samples cannot be empty".to_string());
    }

    let dest_path = macro_sound_path(macro_id)?;
    save_samples_to_wav(&dest_path, samples, sample_rate)?;

    crate::log_info!(
        "Saved microphone recording for macro '{}' ({} samples at {}Hz) to {}",
        macro_id,
        samples.len(),
        sample_rate,
        dest_path.display()
    );

    Ok(dest_path.to_string_lossy().to_string())
}

pub fn delete_macro_sound(macro_id: &str) -> Result<(), String> {
    if let Ok(path) = macro_sound_path(macro_id) {
        if path.exists() {
            let _ = std::fs::remove_file(&path);
            crate::log_info!("Deleted custom macro sound file for '{}'", macro_id);
        }
    }
    Ok(())
}

pub fn clone_macro_sound(source_id: &str, target_id: &str) -> Result<(), String> {
    let src_path = macro_sound_path(source_id)?;
    if src_path.exists() {
        let dest_path = macro_sound_path(target_id)?;
        std::fs::copy(&src_path, &dest_path).map_err(|e| {
            format!(
                "Failed to copy macro sound from '{}' to '{}': {}",
                src_path.display(),
                dest_path.display(),
                e
            )
        })?;
        crate::log_info!("Cloned macro sound from '{}' to '{}'", source_id, target_id);
    }
    Ok(())
}

pub fn cleanup_orphaned_macro_sounds(active_macro_ids: &[String]) -> Result<usize, String> {
    let dir = crate::paths::macro_sounds_dir()?;
    if !dir.exists() {
        return Ok(0);
    }

    let mut removed_count = 0;
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("Failed to read macro sounds dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("wav") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if !active_macro_ids.iter().any(|id| id == stem)
                    && std::fs::remove_file(&path).is_ok()
                {
                    crate::log_info!("Cleaned up orphaned macro sound: {}", path.display());
                    removed_count += 1;
                }
            }
        }
    }

    Ok(removed_count)
}

fn save_samples_to_wav(dest_path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(dest_path, spec)
        .map_err(|e| format!("Failed to create WAV file: {}", e))?;

    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let sample_i16 = (clamped * i16::MAX as f32) as i16;
        writer
            .write_sample(sample_i16)
            .map_err(|e| format!("Failed to write WAV sample: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

    Ok(())
}
