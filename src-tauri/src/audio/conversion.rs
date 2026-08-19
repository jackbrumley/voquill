use hound::{WavSpec, WavWriter};
use rubato::{FftFixedInOut, Resampler};

use super::decode::decode_compressed_audio;

pub fn convert_audio_file_for_whisper(
    data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if is_wav_file(data) {
        let reader = hound::WavReader::new(std::io::Cursor::new(data))?;
        let sample_rate = reader.spec().sample_rate;
        let channels = reader.spec().channels;
        return convert_audio_for_whisper(data, sample_rate, channels);
    }

    crate::log_info!("Audio container: compressed ({} bytes)", data.len());
    let decoded = decode_compressed_audio(data)?;
    crate::log_info!(
        "Decoded: {}Hz, {} channels, {} f32 samples",
        decoded.sample_rate,
        decoded.channels,
        decoded.samples.len(),
    );
    let mono = downmix_to_mono(&decoded.samples, decoded.channels);
    crate::log_info!("Downmixed to {} mono samples", mono.len());
    finalize_captured_audio_for_whisper(&mono, decoded.sample_rate)
}

fn is_wav_file(data: &[u8]) -> bool {
    data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE"
}

fn normalize_peak(samples: &[f32]) -> Vec<f32> {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak < f32::EPSILON {
        return samples.to_vec();
    }
    let gain = (1.0 / peak).min(10.0);
    if (gain - 1.0).abs() < f32::EPSILON {
        return samples.to_vec();
    }
    crate::log_info!("Normalizing audio: peak={:.6}, gain={:.2}", peak, gain);
    samples.iter().map(|s| s * gain).collect()
}

fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

pub fn write_whisper_wav(
    samples: &[i16],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut out = Vec::new();
    {
        let mut writer = WavWriter::new(
            std::io::Cursor::new(&mut out),
            WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }
    Ok(out)
}

/// Parse a full 16kHz mono 16-bit PCM WAV and extract a time-range segment
/// as a valid WAV binary suitable for passing to whisper.
///
/// * `full_wav` — complete 16kHz mono 16-bit WAV bytes
/// * `start_sec` — start time in seconds (clamped to 0)
/// * `end_sec` — end time in seconds (clamped to file duration)
///
/// Segments shorter than 0.5s are padded with silence to avoid whisper choking
/// on tiny fragments from diarization boundaries.
pub fn extract_segment_wav(
    full_wav: &[u8],
    start_sec: f64,
    end_sec: f64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(full_wav))?;
    let spec = reader.spec();

    if spec.channels != 1 || spec.sample_rate != 16000 {
        return Err(format!(
            "Expected 16kHz mono WAV, got {}ch {}Hz",
            spec.channels, spec.sample_rate,
        )
        .into());
    }

    let all_samples: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;

    let total_secs = all_samples.len() as f64 / 16000.0;
    let start = (start_sec.max(0.0).min(total_secs) * 16000.0) as usize;
    let end = (end_sec.max(0.0).min(total_secs) * 16000.0) as usize;
    let end = end.min(all_samples.len());

    let segment = if end <= start || (end - start) < 8000 {
        // Pad short segments to 0.5s (8000 samples at 16kHz)
        let mut padded = Vec::with_capacity(8000);
        if end > start {
            padded.extend_from_slice(&all_samples[start..end]);
        }
        padded.resize(8000, 0);
        padded
    } else {
        all_samples[start..end].to_vec()
    };

    write_whisper_wav(&segment)
}

pub fn convert_audio_for_whisper(
    data: &[u8],
    _rate: u32,
    _chans: u16,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(data))?;
    let spec = reader.spec();

    crate::log_info!(
        "WAV input: rate={}Hz, channels={}, bits={}, format={:?}",
        spec.sample_rate,
        spec.channels,
        spec.bits_per_sample,
        spec.sample_format,
    );

    let total_frames = reader.duration();
    let duration_secs = total_frames as f64 / spec.sample_rate as f64;
    crate::log_info!(
        "WAV details: duration={:.1}s, frames={}, file_size={} bytes",
        duration_secs,
        total_frames,
        data.len(),
    );

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            if spec.bits_per_sample == 16 {
                reader
                    .samples::<i16>()
                    .map(|s| s.unwrap_or(0) as f32 / max)
                    .collect()
            } else {
                reader
                    .samples::<i32>()
                    .map(|s| s.unwrap_or(0) as f32 / max)
                    .collect()
            }
        }
    };
    crate::log_info!(
        "Read {} float samples from {} channels",
        samples.len(),
        spec.channels
    );

    let mono = downmix_to_mono(&samples, spec.channels as usize);
    finalize_captured_audio_for_whisper(&mono, spec.sample_rate)
}

/// Finalize in-memory float audio samples for whisper:
/// - Skips resampling if the stream is already 16,000Hz (0ms latency).
/// - Resamples with band-limited FFT if not 16,000Hz.
/// - Normalizes peak amplitude up to 1.0 (max 10x gain).
/// - Writes single 16-bit mono 16kHz PCM WAV.
pub fn finalize_captured_audio_for_whisper(
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let resampled = if sample_rate != 16000 {
        crate::log_info!(
            "Resampling audio: {}Hz -> 16000Hz ({} samples)",
            sample_rate,
            samples.len()
        );
        resample_audio_f32(samples, sample_rate, 16000)
    } else {
        samples.to_vec()
    };

    let normalized = normalize_peak(&resampled);
    let peak = normalized.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = if normalized.is_empty() {
        0.0
    } else {
        (normalized.iter().map(|s| s * s).sum::<f32>() / normalized.len() as f32).sqrt()
    };
    crate::log_info!(
        "Output audio: {} samples ({:.1}s at 16kHz), peak={:.4}, rms={:.6}",
        normalized.len(),
        normalized.len() as f64 / 16000.0,
        peak,
        rms,
    );
    let samples_i16: Vec<i16> = normalized
        .iter()
        .map(|&sample| float_to_i16(sample))
        .collect();
    write_whisper_wav(&samples_i16)
}

pub fn resample_audio_f32(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let mut resampler = FftFixedInOut::<f32>::new(from as usize, to as usize, 1024, 1);
    let frames_needed = resampler.nbr_frames_needed();
    let expected_output_len = (samples.len() as f64 * to as f64 / from as f64).round() as usize;
    let mut output = Vec::with_capacity(expected_output_len + frames_needed);

    let mut pos = 0;
    while pos < samples.len() {
        let end = (pos + frames_needed).min(samples.len());
        let chunk = &samples[pos..end];
        let input_chunk = if chunk.len() < frames_needed {
            let mut padded = chunk.to_vec();
            padded.resize(frames_needed, 0.0);
            padded
        } else {
            chunk.to_vec()
        };

        let resampled = resampler
            .process(&[input_chunk])
            .expect("Rubato FFT resampling failed");
        if let Some(chan) = resampled.into_iter().next() {
            output.extend_from_slice(&chan);
        }
        pos += frames_needed;
    }

    output.truncate(expected_output_len);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_same_rate_is_identity() {
        let input = vec![0.1, -0.2, 0.3, -0.4];
        let output = resample_audio_f32(&input, 16000, 16000);
        assert_eq!(input, output);
    }

    #[test]
    fn resample_empty_is_empty() {
        let output = resample_audio_f32(&[], 44100, 16000);
        assert!(output.is_empty());
    }

    #[test]
    fn resample_short_audio() {
        let input = vec![0.1; 100];
        let output = resample_audio_f32(&input, 44100, 16000);
        let expected_len = (100.0 * 16000.0 / 44100.0_f64).round() as usize;
        assert_eq!(output.len(), expected_len);
    }

    #[test]
    fn resample_1s_audio_44100_to_16000() {
        let input = vec![0.05; 44148];
        let output = resample_audio_f32(&input, 44100, 16000);
        let expected_len = (44148.0 * 16000.0 / 44100.0_f64).round() as usize;
        assert_eq!(output.len(), expected_len);
    }

    #[test]
    fn resample_exact_failing_length_35s() {
        // The exact sample count from the crash log: 1,585,268 samples (35.9s at 44.1kHz)
        let input = vec![0.02; 1585268];
        let output = resample_audio_f32(&input, 44100, 16000);
        let expected_len = (1585268.0 * 16000.0 / 44100.0_f64).round() as usize;
        assert_eq!(output.len(), expected_len);
    }

    #[test]
    fn resample_48k_to_16k() {
        let input = vec![0.01; 48000 * 5]; // 5 seconds of 48kHz
        let output = resample_audio_f32(&input, 48000, 16000);
        assert_eq!(output.len(), 16000 * 5);
    }

    #[test]
    fn finalize_captured_audio_16k_direct_path() {
        let input = vec![0.5; 16000]; // 1 second of 16kHz
        let wav_bytes =
            finalize_captured_audio_for_whisper(&input, 16000).expect("finalize failed");
        let reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.duration(), 16000);
    }

    #[test]
    fn finalize_captured_audio_44100_resampled_path() {
        let input = vec![0.3; 44100]; // 1 second of 44.1kHz
        let wav_bytes =
            finalize_captured_audio_for_whisper(&input, 44100).expect("finalize failed");
        let reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.duration(), 16000);
    }
}
