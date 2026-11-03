//! Single owner of every playback stream Voquill opens.
//!
//! Sounds play through `AudioPlayer` on a `PlaybackSlot`. Each slot plays one
//! sound at a time: starting a sound ends whatever the slot was playing, so a
//! new mic test replaces a history clip but never cuts off a voice macro chime.
//! Every sound's `on_end` runs exactly once, whether it finished, was replaced
//! or was stopped, so callers can tie UI state to it without extra bookkeeping.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};

use super::conversion::resample_linear;
use super::decode::DecodedAudio;

const PLAYBACK_GAIN: f32 = 0.70;

/// Silence written after a clip before it counts as finished, so audio still
/// queued in the device buffer is heard rather than cut off when the stream
/// closes, and "finished" is never reported early.
const END_OF_CLIP_TAIL_MS: u32 = 150;

/// Sample formats the playback callback writes, in preference order.
const PLAYBACK_SAMPLE_FORMATS: [SampleFormat; 2] = [SampleFormat::F32, SampleFormat::I16];

/// Mono audio ready to play.
#[derive(Clone)]
pub struct AudioClip {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl AudioClip {
    pub fn from_decoded(decoded: DecodedAudio) -> Self {
        Self {
            samples: downmix_to_mono(decoded.samples, decoded.channels),
            sample_rate: decoded.sample_rate,
        }
    }

    pub fn from_wav_bytes(wav_bytes: &[u8]) -> Result<Self, String> {
        let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes))
            .map_err(|error| format!("Failed to read WAV audio: {}", error))?;
        let spec = reader.spec();
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max_value = (1_i64 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .filter_map(|sample| sample.ok())
                    .map(|sample| (sample as f32 / max_value).clamp(-1.0, 1.0))
                    .collect()
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .filter_map(|sample| sample.ok())
                .map(|sample| sample.clamp(-1.0, 1.0))
                .collect(),
        };
        Ok(Self {
            samples: downmix_to_mono(samples, spec.channels as usize),
            sample_rate: spec.sample_rate,
        })
    }
}

fn downmix_to_mono(samples: Vec<f32>, channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples;
    }
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Independent playback lanes. A slot plays one sound at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaybackSlot {
    /// User-requested listening: mic test playback and history recordings.
    Preview,
    /// Voice macro chimes, recorded sounds and TTS previews.
    MacroFeedback,
}

struct ActivePlayback {
    id: u64,
    _stream: cpal::Stream,
    on_end: Box<dyn FnOnce() + Send>,
}

#[derive(Default)]
pub struct AudioPlayer {
    active: Mutex<HashMap<PlaybackSlot, ActivePlayback>>,
    next_playback_id: AtomicU64,
}

impl AudioPlayer {
    /// Plays `clip` on `slot`, ending whatever the slot was playing.
    /// `device_id` is the configured playback device (`None` / `"default"`
    /// for the system default).
    pub fn play(
        self: &Arc<Self>,
        slot: PlaybackSlot,
        clip: AudioClip,
        device_id: Option<String>,
        on_end: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        let playback_id = self.next_playback_id.fetch_add(1, Ordering::Relaxed) + 1;
        let device = super::device::lookup_output_device(device_id)?;

        // Held until the new playback is registered, so a clip that finishes
        // before `play` returns still finds its entry to end.
        let mut active = self.active.lock().unwrap();
        let replaced = active.remove(&slot);

        let player = Arc::downgrade(self);
        let stream = open_clip_stream(&device, clip, move || {
            end_from_audio_thread(player, slot, playback_id)
        });

        let result = match stream {
            Ok(stream) => {
                active.insert(
                    slot,
                    ActivePlayback {
                        id: playback_id,
                        _stream: stream,
                        on_end: Box::new(on_end),
                    },
                );
                Ok(())
            }
            Err(error) => Err(format!("Playback failed: {}", error)),
        };
        drop(active);

        if let Some(replaced) = replaced {
            (replaced.on_end)();
        }
        result
    }

    /// Stops whatever `slot` is playing.
    pub fn stop(&self, slot: PlaybackSlot) {
        let stopped = self.active.lock().unwrap().remove(&slot);
        if let Some(stopped) = stopped {
            (stopped.on_end)();
        }
    }

    pub fn stop_all(&self) {
        let stopped: Vec<ActivePlayback> = self
            .active
            .lock()
            .unwrap()
            .drain()
            .map(|(_, playback)| playback)
            .collect();
        for playback in stopped {
            (playback.on_end)();
        }
    }

    fn end_if_current(&self, slot: PlaybackSlot, playback_id: u64) {
        let finished = {
            let mut active = self.active.lock().unwrap();
            match active.get(&slot) {
                Some(playback) if playback.id == playback_id => active.remove(&slot),
                _ => None,
            }
        };
        if let Some(finished) = finished {
            (finished.on_end)();
        }
    }
}

/// Called from the stream's audio callback when the clip has played out. A
/// stream must not be dropped from its own callback, so the end is handled on
/// a separate thread.
fn end_from_audio_thread(player: Weak<AudioPlayer>, slot: PlaybackSlot, playback_id: u64) {
    std::thread::spawn(move || {
        if let Some(player) = player.upgrade() {
            player.end_if_current(slot, playback_id);
        }
    });
}

/// The device's default output config, or the same rate and channel layout
/// in a format the callback writes when the default is I24/I32/etc.
fn select_playback_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    let default_config = device.default_output_config().map_err(|error| {
        format!(
            "Failed to read the output device's default config: {}",
            error
        )
    })?;
    if PLAYBACK_SAMPLE_FORMATS.contains(&default_config.sample_format()) {
        return Ok(default_config);
    }

    let ranges: Vec<cpal::SupportedStreamConfigRange> = device
        .supported_output_configs()
        .map_err(|error| format!("Failed to query output configs: {}", error))?
        .collect();
    for format in PLAYBACK_SAMPLE_FORMATS {
        if let Some(range) = ranges.iter().find(|range| {
            range.sample_format() == format
                && range.channels() == default_config.channels()
                && range.contains_rate(default_config.sample_rate())
        }) {
            return Ok(range.with_sample_rate(default_config.sample_rate()));
        }
    }

    Err(format!(
        "Output device offers no F32 or I16 config at {}Hz with {} channels (default format {:?})",
        default_config.sample_rate(),
        default_config.channels(),
        default_config.sample_format()
    ))
}

fn open_clip_stream(
    device: &cpal::Device,
    clip: AudioClip,
    on_finished: impl FnOnce() + Send + 'static,
) -> Result<cpal::Stream, String> {
    let config = select_playback_config(device)?;
    let mut stream_config: StreamConfig = config.config();
    stream_config.buffer_size = super::device::stream_buffer_size(device, &config);
    let samples = resample_linear(&clip.samples, clip.sample_rate, stream_config.sample_rate);

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_clip_stream::<f32>(device, stream_config, samples, on_finished),
        SampleFormat::I16 => build_clip_stream::<i16>(device, stream_config, samples, on_finished),
        other => return Err(format!("Unsupported playback sample format {:?}", other)),
    }?;
    stream
        .play()
        .map_err(|error| format!("Failed to start playback stream: {}", error))?;
    Ok(stream)
}

fn build_clip_stream<T>(
    device: &cpal::Device,
    stream_config: StreamConfig,
    samples: Vec<f32>,
    on_finished: impl FnOnce() + Send + 'static,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = stream_config.channels as usize;
    let finished_after_frames =
        samples.len() + (stream_config.sample_rate * END_OF_CLIP_TAIL_MS / 1000) as usize;
    let mut position = 0_usize;
    let mut on_finished = Some(on_finished);

    device
        .build_output_stream(
            stream_config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let value = samples
                        .get(position)
                        .map(|sample| (sample * PLAYBACK_GAIN).clamp(-1.0, 1.0))
                        .unwrap_or(0.0);
                    frame.fill(T::from_sample(value));
                    position += 1;
                }
                if position >= finished_after_frames {
                    if let Some(on_finished) = on_finished.take() {
                        on_finished();
                    }
                }
            },
            |error| crate::log_warn!("Playback stream error: {}", error),
            None,
        )
        .map_err(|error| format!("Failed to open playback stream: {}", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_bytes(channels: u16, frames: &[[i16; 2]]) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for frame in frames {
            for sample in frame.iter().take(channels as usize) {
                writer.write_sample(*sample).unwrap();
            }
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn stereo_wav_is_downmixed_to_one_sample_per_frame() {
        let clip =
            AudioClip::from_wav_bytes(&wav_bytes(2, &[[16384, 0], [-16384, -16384]])).unwrap();
        assert_eq!(clip.sample_rate, 16000);
        assert_eq!(clip.samples.len(), 2);
        assert!((clip.samples[0] - 0.25).abs() < 1e-3);
        assert!((clip.samples[1] + 0.5).abs() < 1e-3);
    }

    #[test]
    fn mono_wav_keeps_its_samples() {
        let clip =
            AudioClip::from_wav_bytes(&wav_bytes(1, &[[16384, 0], [0, 0], [-16384, 0]])).unwrap();
        assert_eq!(clip.samples.len(), 3);
    }

    #[test]
    fn invalid_wav_is_an_error() {
        assert!(AudioClip::from_wav_bytes(b"not a wav file").is_err());
    }
}
