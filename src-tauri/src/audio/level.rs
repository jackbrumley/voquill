//! Speech level: how loud the speech in a recording is, and the gain that
//! brings it to a level every speech engine hears clearly. Quiet
//! microphones are boosted (up to `MAX_GAIN_DB`), loud ones are kept below
//! clipping, and a recording that holds nothing but the noise floor is left
//! alone so silence is never amplified into hiss.

/// Speech is brought to this RMS level (about -15 dBFS).
const TARGET_SPEECH_RMS: f32 = 0.18;
/// The most a recording is boosted (+30 dB).
const MAX_GAIN_DB: f32 = 30.0;
/// A recording whose loudest sample stays below this (-60 dBFS) is silence.
const SILENCE_PEAK: f32 = 0.001;
/// Samples above this are softly limited instead of clipped.
const LIMIT_THRESHOLD: f32 = 0.95;
/// Speech level is measured in 20 ms frames.
const FRAME_SECONDS: f32 = 0.020;
/// The frame at this loudness rank stands for the speech: well above the
/// pauses between words, yet below a single click or bump.
const SPEECH_FRAME_PERCENTILE: f32 = 0.95;

/// What `measure` found in a recording.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechLevel {
    /// The loudest sample.
    pub peak: f32,
    /// RMS of the speech (the loud frames), not of the whole recording.
    pub speech_rms: f32,
    /// The gain `normalize` applies.
    pub gain: f32,
    /// True when even the largest boost leaves the speech well below the
    /// target: the microphone (or its gain) is set too low.
    pub is_too_quiet: bool,
}

/// Measures the speech level of `samples` and decides the gain.
pub fn measure(samples: &[f32], sample_rate: u32) -> SpeechLevel {
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0f32, f32::max);
    if peak < SILENCE_PEAK {
        return SpeechLevel {
            peak,
            speech_rms: 0.0,
            gain: 1.0,
            is_too_quiet: true,
        };
    }

    let speech_rms = speech_rms(samples, sample_rate);
    let max_gain = 10f32.powf(MAX_GAIN_DB / 20.0);
    let speech_gain = TARGET_SPEECH_RMS / speech_rms.max(f32::EPSILON);
    let headroom_gain = LIMIT_THRESHOLD / loud_sample_level(samples);
    let gain = speech_gain.min(headroom_gain).min(max_gain);

    SpeechLevel {
        peak,
        speech_rms,
        gain,
        is_too_quiet: speech_rms * max_gain < TARGET_SPEECH_RMS / 2.0,
    }
}

/// Applies the gain `measure` decides, softly limiting the rare samples
/// that would otherwise clip.
pub fn normalize(samples: &[f32], sample_rate: u32) -> (Vec<f32>, SpeechLevel) {
    let level = measure(samples, sample_rate);
    if (level.gain - 1.0).abs() < f32::EPSILON {
        return (samples.to_vec(), level);
    }

    crate::log_info!(
        "Speech level: peak={:.4}, speech_rms={:.4}, gain={:.2} ({:+.1} dB){}",
        level.peak,
        level.speech_rms,
        level.gain,
        20.0 * level.gain.log10(),
        if level.is_too_quiet {
            ", microphone too quiet"
        } else {
            ""
        },
    );

    let adjusted = samples
        .iter()
        .map(|&sample| soft_limit(sample * level.gain))
        .collect();
    (adjusted, level)
}

fn speech_rms(samples: &[f32], sample_rate: u32) -> f32 {
    let frame_len = ((sample_rate as f32 * FRAME_SECONDS).round() as usize).max(1);
    let mut frame_levels: Vec<f32> = samples
        .chunks(frame_len)
        .map(|frame| {
            (frame.iter().map(|sample| sample * sample).sum::<f32>() / frame.len() as f32).sqrt()
        })
        .collect();
    frame_levels.sort_by(f32::total_cmp);
    let rank = (frame_levels.len() as f32 * SPEECH_FRAME_PERCENTILE) as usize;
    frame_levels[rank.min(frame_levels.len() - 1)]
}

/// The 99.5th percentile sample: the level speech peaks reach, ignoring
/// the odd click that would otherwise hold the gain down.
fn loud_sample_level(samples: &[f32]) -> f32 {
    let mut magnitudes: Vec<f32> = samples.iter().map(|sample| sample.abs()).collect();
    magnitudes.sort_by(f32::total_cmp);
    let rank = (magnitudes.len() as f64 * 0.995).round() as usize;
    magnitudes[rank.min(magnitudes.len() - 1)].max(f32::EPSILON)
}

fn soft_limit(sample: f32) -> f32 {
    let magnitude = sample.abs();
    if magnitude <= LIMIT_THRESHOLD {
        return sample;
    }
    let over = (magnitude - LIMIT_THRESHOLD) / (1.0 - LIMIT_THRESHOLD);
    sample.signum() * (LIMIT_THRESHOLD + (1.0 - LIMIT_THRESHOLD) * (over / (over + 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16000;

    fn tone(amplitude: f32, seconds: f32) -> Vec<f32> {
        (0..(RATE as f32 * seconds) as usize)
            .map(|index| amplitude * (index as f32 * 0.1).sin())
            .collect()
    }

    #[test]
    fn boosts_a_quiet_microphone_to_the_target() {
        let (adjusted, level) = normalize(&tone(0.02, 1.0), RATE);
        assert!(level.gain > 5.0);
        assert!(!level.is_too_quiet);
        let adjusted_rms = speech_rms(&adjusted, RATE);
        assert!(
            (adjusted_rms - TARGET_SPEECH_RMS).abs() < 0.02,
            "{adjusted_rms}"
        );
    }

    #[test]
    fn caps_the_boost_and_reports_a_very_quiet_microphone() {
        let level = measure(&tone(0.002, 1.0), RATE);
        assert!((20.0 * level.gain.log10() - MAX_GAIN_DB).abs() < 0.01);
        assert!(level.is_too_quiet);
    }

    #[test]
    fn leaves_silence_alone() {
        let silence = vec![0.0005f32; RATE as usize];
        let (adjusted, level) = normalize(&silence, RATE);
        assert_eq!(level.gain, 1.0);
        assert!(level.is_too_quiet);
        assert_eq!(adjusted, silence);
    }

    #[test]
    fn measures_the_speech_not_the_pauses() {
        let mut recording = vec![0.0f32; RATE as usize * 3];
        recording.extend(tone(0.05, 1.0));
        let level = measure(&recording, RATE);
        assert!(level.speech_rms > 0.03, "{}", level.speech_rms);
    }

    #[test]
    fn never_clips_loud_speech() {
        let (adjusted, _) = normalize(&tone(0.99, 1.0), RATE);
        assert!(adjusted.iter().all(|sample| sample.abs() < 1.0));
    }
}
