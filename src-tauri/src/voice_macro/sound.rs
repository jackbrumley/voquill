use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use crate::audio::decode::{decode_compressed_audio, DecodedAudio};
use crate::config::{MacroSoundMode, VoiceMacroCommand};

const MACRO_TRIGGER_SOUND_BYTES: &[u8] = include_bytes!("../../sounds/macro_trigger.mp3");

static CACHED_SOUND: OnceLock<Option<DecodedAudio>> = OnceLock::new();

fn get_cached_sound() -> Option<&'static DecodedAudio> {
    CACHED_SOUND
        .get_or_init(
            || match decode_compressed_audio(MACRO_TRIGGER_SOUND_BYTES) {
                Ok(decoded) => {
                    crate::log_info!(
                        "Voice Macro sound decoded successfully ({} samples at {}Hz)",
                        decoded.samples.len(),
                        decoded.sample_rate
                    );
                    Some(decoded)
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

pub fn play_macro_trigger_sound(playback_device: Option<String>) {
    if let Some(decoded) = get_cached_sound() {
        let samples = decoded.samples.clone();
        let sample_rate = decoded.sample_rate;
        std::thread::spawn(move || {
            match crate::audio::playback::play_audio(samples, sample_rate, playback_device, || {}) {
                Ok(stream) => {
                    std::thread::sleep(Duration::from_millis(1500));
                    drop(stream);
                }
                Err(e) => {
                    crate::log_warn!("Voice Macro playback error: {}", e);
                }
            }
        });
    }
}

pub fn play_macro_sound(
    command: &VoiceMacroCommand,
    playback_device: Option<String>,
    global_feedback_enabled: bool,
) {
    match command.sound_mode {
        MacroSoundMode::None => {
            // Explicitly muted
        }
        MacroSoundMode::Default => {
            if global_feedback_enabled {
                play_macro_trigger_sound(playback_device);
            }
        }
        MacroSoundMode::Tts | MacroSoundMode::CustomFile | MacroSoundMode::MicRecording => {
            if let Ok(path) = macro_sound_path(&command.id) {
                if path.exists() {
                    let _ = play_macro_sound_file(&path, playback_device.clone());
                    return;
                }
            }
            // Fallback to default chirp if custom audio missing on disk
            if global_feedback_enabled {
                play_macro_trigger_sound(playback_device);
            }
        }
    }
}

pub fn play_macro_sound_file(
    file_path: &Path,
    playback_device: Option<String>,
) -> Result<(), String> {
    let bytes =
        std::fs::read(file_path).map_err(|e| format!("Failed to read macro sound file: {}", e))?;

    let decoded = decode_compressed_audio(&bytes)
        .map_err(|e| format!("Failed to decode macro sound file: {}", e))?;

    let samples = decoded.samples;
    let sample_rate = decoded.sample_rate;
    let duration_ms = (samples.len() as f32 / sample_rate as f32 * 1000.0) as u64 + 500;

    std::thread::spawn(move || {
        match crate::audio::playback::play_audio(samples, sample_rate, playback_device, || {}) {
            Ok(stream) => {
                std::thread::sleep(Duration::from_millis(duration_ms));
                drop(stream);
            }
            Err(e) => {
                crate::log_warn!("Voice Macro file playback error: {}", e);
            }
        }
    });

    Ok(())
}

pub fn import_macro_audio_file(macro_id: &str, source_path: &str) -> Result<String, String> {
    let src_bytes = std::fs::read(source_path)
        .map_err(|e| format!("Failed to read source audio file '{}': {}", source_path, e))?;

    let decoded = decode_compressed_audio(&src_bytes)
        .map_err(|e| format!("Failed to decode source audio file: {}", e))?;

    let dest_path = macro_sound_path(macro_id)?;

    save_samples_to_wav(&dest_path, &decoded.samples, decoded.sample_rate)?;
    crate::log_info!(
        "Imported custom macro audio for '{}' ({} samples at {}Hz) to {}",
        macro_id,
        decoded.samples.len(),
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
