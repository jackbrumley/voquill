use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct VadFrameResult {
    pub raw_rms: f32,
    pub smoothed_rms: f32,
    pub is_speaking: bool,
    pub speech_just_started: bool,
    pub speech_just_ended: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct MicVolumePayload {
    pub volume: f32,
    pub is_triggered: bool,
}

pub struct VoiceActivityDetector {
    pub smoothed_rms: f32,
    pub is_speaking: bool,
    onset_counter: usize,
    silence_frames: usize,
    onset_required_frames: usize,
    hangover_required_frames: usize,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceActivityDetector {
    pub fn new() -> Self {
        Self {
            smoothed_rms: 0.0,
            is_speaking: false,
            onset_counter: 0,
            silence_frames: 0,
            onset_required_frames: 2,     // ~60ms onset
            hangover_required_frames: 13, // ~400ms hangover at 30ms frames
        }
    }

    pub fn process_frame(&mut self, frame_samples: &[f32], open_threshold: f32) -> VadFrameResult {
        let energy: f32 =
            frame_samples.iter().map(|&s| s * s).sum::<f32>() / frame_samples.len().max(1) as f32;
        let raw_rms = energy.sqrt();

        // Envelope follower: Fast attack (0.7), smooth release (0.15)
        if raw_rms > self.smoothed_rms {
            self.smoothed_rms = 0.7 * raw_rms + 0.3 * self.smoothed_rms;
        } else {
            self.smoothed_rms = 0.15 * raw_rms + 0.85 * self.smoothed_rms;
        }

        let close_threshold = open_threshold * 0.60;
        let mut speech_just_started = false;
        let mut speech_just_ended = false;

        if !self.is_speaking {
            if self.smoothed_rms >= open_threshold {
                self.onset_counter += 1;
                if self.onset_counter >= self.onset_required_frames {
                    self.is_speaking = true;
                    self.onset_counter = 0;
                    self.silence_frames = 0;
                    speech_just_started = true;
                }
            } else {
                self.onset_counter = 0;
            }
        } else if self.smoothed_rms >= close_threshold {
            self.silence_frames = 0;
        } else {
            self.silence_frames += 1;
            if self.silence_frames >= self.hangover_required_frames {
                self.is_speaking = false;
                self.silence_frames = 0;
                self.onset_counter = 0;
                speech_just_ended = true;
            }
        }

        VadFrameResult {
            raw_rms,
            smoothed_rms: self.smoothed_rms,
            is_speaking: self.is_speaking,
            speech_just_started,
            speech_just_ended,
        }
    }

    pub fn reset(&mut self) {
        self.smoothed_rms = 0.0;
        self.is_speaking = false;
        self.onset_counter = 0;
        self.silence_frames = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_onset_requires_two_frames() {
        let mut vad = VoiceActivityDetector::new();
        let loud_samples = vec![0.08f32; 480];
        let threshold = 0.035;

        // Frame 1: onset_counter reaches 1, not speaking yet
        let res1 = vad.process_frame(&loud_samples, threshold);
        assert!(!res1.is_speaking);
        assert!(!res1.speech_just_started);

        // Frame 2: onset_counter reaches 2, speaking starts!
        let res2 = vad.process_frame(&loud_samples, threshold);
        assert!(res2.is_speaking);
        assert!(res2.speech_just_started);
    }

    #[test]
    fn test_vad_hysteresis_holds_across_dips() {
        let mut vad = VoiceActivityDetector::new();
        let loud_samples = vec![0.08f32; 480];
        let threshold = 0.035;

        vad.process_frame(&loud_samples, threshold);
        vad.process_frame(&loud_samples, threshold);
        assert!(vad.is_speaking);

        // Dip below open_threshold (0.035) but above close_threshold (0.021)
        let dip_samples = vec![0.025f32; 480];
        let res = vad.process_frame(&dip_samples, threshold);
        assert!(res.is_speaking);
        assert_eq!(vad.silence_frames, 0);
    }

    #[test]
    fn test_vad_hangover_ends_after_silent_frames() {
        let mut vad = VoiceActivityDetector::new();
        let loud_samples = vec![0.08f32; 480];
        let silent_samples = vec![0.001f32; 480];
        let threshold = 0.035;

        vad.process_frame(&loud_samples, threshold);
        vad.process_frame(&loud_samples, threshold);
        assert!(vad.is_speaking);

        let mut ended = false;
        for _ in 0..30 {
            let res = vad.process_frame(&silent_samples, threshold);
            if res.speech_just_ended {
                ended = true;
                assert!(!res.is_speaking);
                break;
            }
        }
        assert!(ended);
    }
}
