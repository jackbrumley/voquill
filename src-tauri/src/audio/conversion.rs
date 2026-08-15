use hound::{WavSpec, WavWriter};

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
    let resampled = resample_audio_f32(&mono, decoded.sample_rate, 16000);
    crate::log_info!(
        "Resampled {} -> {} samples ({}Hz -> 16000Hz)",
        mono.len(),
        resampled.len(),
        decoded.sample_rate,
    );
    let normalized = normalize_peak(&resampled);
    let peak = normalized.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = (normalized.iter().map(|s| s * s).sum::<f32>() / normalized.len() as f32).sqrt();
    crate::log_info!(
        "Output audio: {} samples ({:.1}s at 16kHz), peak={:.4}, rms={:.6}",
        normalized.len(),
        normalized.len() as f64 / 16000.0,
        peak,
        rms,
    );
    let samples: Vec<i16> = normalized
        .iter()
        .map(|&sample| float_to_i16(sample))
        .collect();
    write_whisper_wav(&samples)
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

    let mono = if spec.channels > 1 {
        let m = samples
            .chunks_exact(spec.channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / spec.channels as f32)
            .collect::<Vec<_>>();
        crate::log_info!(
            "Downmixed {} channels -> {} mono samples",
            spec.channels,
            m.len()
        );
        m
    } else {
        samples
    };

    let resampled = if spec.sample_rate != 16000 {
        let r = resample_audio_f32(&mono, spec.sample_rate, 16000);
        crate::log_info!(
            "Resampled {} -> {} samples ({}Hz -> 16000Hz)",
            mono.len(),
            r.len(),
            spec.sample_rate,
        );
        r
    } else {
        mono
    };

    let normalized = normalize_peak(&resampled);

    let peak = normalized.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = (normalized.iter().map(|s| s * s).sum::<f32>() / normalized.len() as f32).sqrt();
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
    if from == to {
        return samples.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = pos - idx as f64;
        if idx + 1 < samples.len() {
            let s1 = samples[idx] as f64;
            let s2 = samples[idx + 1] as f64;
            out.push((s1 + (s2 - s1) * frac) as f32);
        } else if idx < samples.len() {
            out.push(samples[idx]);
        }
    }
    out
}

pub fn process_sample(s: f32) -> i16 {
    let clipped = soft_clip(s);
    (clipped * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn soft_clip(x: f32) -> f32 {
    if x.abs() <= 0.7 {
        x
    } else if x > 0.7 {
        0.7 + 0.3 * ((x - 0.7) / 0.3).tanh()
    } else {
        -0.7 - 0.3 * ((-x - 0.7) / 0.3).tanh()
    }
}
