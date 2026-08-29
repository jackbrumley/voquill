use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use crate::audio::decode::{decode_compressed_audio, DecodedAudio};
use crate::config::{MacroSoundMode, VoiceMacroCommand};

const MACRO_TRIGGER_SOUND_BYTES: &[u8] = include_bytes!("../../sounds/macro_trigger.mp3");

static CACHED_SOUND: OnceLock<Option<DecodedAudio>> = OnceLock::new();
static ACTIVE_SOUND_STREAM: std::sync::Mutex<Option<cpal::Stream>> = std::sync::Mutex::new(None);
static ACTIVE_CANCEL_FLAG: std::sync::Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> =
    std::sync::Mutex::new(None);
static PLAYBACK_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn stop_macro_sound_playback() {
    PLAYBACK_GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut lock) = ACTIVE_CANCEL_FLAG.lock() {
        if let Some(flag) = lock.take() {
            crate::log_info!("Signaling instant audio mute/cancellation");
            flag.store(true, Ordering::SeqCst);
        }
    }
    if let Ok(mut lock) = ACTIVE_SOUND_STREAM.lock() {
        if lock.is_some() {
            crate::log_info!("Stopping active macro sound playback stream");
            *lock = None;
        }
    }
}

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
        let samples = if decoded.channels > 1 {
            let chans = decoded.channels;
            decoded
                .samples
                .chunks_exact(chans)
                .map(|frame| frame.iter().sum::<f32>() / chans as f32)
                .collect::<Vec<f32>>()
        } else {
            decoded.samples.clone()
        };
        let sample_rate = decoded.sample_rate;
        let duration_ms = (samples.len() as f32 / sample_rate as f32 * 1000.0) as u64 + 400;

        stop_macro_sound_playback();
        let current_gen = PLAYBACK_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        if let Ok(mut lock) = ACTIVE_CANCEL_FLAG.lock() {
            *lock = Some(cancel_flag.clone());
        }

        match crate::audio::playback::play_audio_cancellable(
            samples,
            sample_rate,
            playback_device,
            cancel_flag,
            || {},
        ) {
            Ok(stream) => {
                if let Ok(mut lock) = ACTIVE_SOUND_STREAM.lock() {
                    *lock = Some(stream);
                }

                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(duration_ms));
                    if PLAYBACK_GENERATION.load(Ordering::SeqCst) == current_gen {
                        if let Ok(mut lock) = ACTIVE_SOUND_STREAM.lock() {
                            if PLAYBACK_GENERATION.load(Ordering::SeqCst) == current_gen {
                                *lock = None;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                crate::log_warn!("Voice Macro trigger playback error: {}", e);
            }
        }
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
    crate::log_info!("Loading macro sound file from {}", file_path.display());
    let bytes =
        std::fs::read(file_path).map_err(|e| format!("Failed to read macro sound file: {}", e))?;

    let decoded = decode_compressed_audio(&bytes)
        .map_err(|e| format!("Failed to decode macro sound file: {}", e))?;

    let samples = if decoded.channels > 1 {
        let chans = decoded.channels;
        decoded
            .samples
            .chunks_exact(chans)
            .map(|frame| frame.iter().sum::<f32>() / chans as f32)
            .collect::<Vec<f32>>()
    } else {
        decoded.samples
    };
    let sample_rate = decoded.sample_rate;
    let duration_ms = (samples.len() as f32 / sample_rate as f32 * 1000.0) as u64 + 400;

    stop_macro_sound_playback();
    let current_gen = PLAYBACK_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    if let Ok(mut lock) = ACTIVE_CANCEL_FLAG.lock() {
        *lock = Some(cancel_flag.clone());
    }

    crate::log_info!(
        "Spawning sound playback (samples={}, rate={}Hz, dur={}ms, gen={})",
        samples.len(),
        sample_rate,
        duration_ms,
        current_gen
    );

    match crate::audio::playback::play_audio_cancellable(
        samples,
        sample_rate,
        playback_device,
        cancel_flag,
        || {},
    ) {
        Ok(stream) => {
            if let Ok(mut lock) = ACTIVE_SOUND_STREAM.lock() {
                *lock = Some(stream);
            }

            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(duration_ms));
                if PLAYBACK_GENERATION.load(Ordering::SeqCst) == current_gen {
                    if let Ok(mut lock) = ACTIVE_SOUND_STREAM.lock() {
                        if PLAYBACK_GENERATION.load(Ordering::SeqCst) == current_gen {
                            *lock = None;
                            crate::log_info!("Playback finished cleanly for gen={}", current_gen);
                        }
                    }
                }
            });

            Ok(())
        }
        Err(e) => {
            let err_msg = format!("Voice Macro playback error: {}", e);
            crate::log_warn!("{}", err_msg);
            Err(err_msg)
        }
    }
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
