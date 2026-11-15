//! Turns a live audio stream into spoken phrases, the way the voice macro
//! listener hears them. The single owner of that rule, so the listener, the
//! spoken macro tester and the mic test meter can never disagree about what
//! counts as speech.
//!
//! Audio is processed in 30 ms frames through `VoiceActivityDetector`. A
//! phrase starts with ~210 ms of lead-in (so the first syllable is kept) and
//! ends when speech stops, or at 4 s at the latest. Phrases shorter than
//! 0.3 s are dropped as noise.

use super::vad::{VadFrameResult, VoiceActivityDetector};
use std::collections::VecDeque;

const FRAME_SECONDS: f32 = 0.03;
const LEAD_IN_FRAMES: usize = 7;
const MAX_PHRASE_SECONDS: f32 = 4.0;
const MIN_PHRASE_SECONDS: f32 = 0.30;

/// How a phrase ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhraseEnd {
    SpeechStopped,
    MaxLength,
}

#[derive(Clone, Debug)]
pub struct SpeechPhrase {
    pub samples: Vec<f32>,
    pub duration_secs: f32,
    /// The loudest smoothed level reached during the phrase.
    pub peak_level: f32,
    pub ended_by: PhraseEnd,
}

/// The result of one 30 ms frame.
pub struct FrameOutcome {
    pub level: VadFrameResult,
    /// A phrase that this frame completed.
    pub phrase: Option<SpeechPhrase>,
}

pub struct SpeechSegmenter {
    sample_rate: u32,
    frame_size: usize,
    frame: Vec<f32>,
    lead_in: VecDeque<Vec<f32>>,
    phrase: Vec<f32>,
    peak_level: f32,
    detector: VoiceActivityDetector,
}

impl SpeechSegmenter {
    pub fn new(sample_rate: u32) -> Self {
        let frame_size = ((sample_rate as f32 * FRAME_SECONDS) as usize).max(1);
        Self {
            sample_rate,
            frame_size,
            frame: Vec::with_capacity(frame_size),
            lead_in: VecDeque::with_capacity(LEAD_IN_FRAMES),
            phrase: Vec::new(),
            peak_level: 0.0,
            detector: VoiceActivityDetector::new(),
        }
    }

    /// Adds one sample; returns the frame it completes, if any. Callers
    /// decide whether to `process_frame` it (or `reset` while paused).
    pub fn push_sample(&mut self, sample: f32) -> Option<Vec<f32>> {
        self.frame.push(sample);
        if self.frame.len() < self.frame_size {
            return None;
        }
        Some(std::mem::replace(
            &mut self.frame,
            Vec::with_capacity(self.frame_size),
        ))
    }

    /// Runs one frame through the detector with the activation `threshold`.
    pub fn process_frame(&mut self, frame: &[f32], threshold: f32) -> FrameOutcome {
        let level = self.detector.process_frame(frame, threshold);
        let mut phrase = None;

        if level.speech_just_started {
            self.phrase.clear();
            for lead in &self.lead_in {
                self.phrase.extend_from_slice(lead);
            }
            self.phrase.extend_from_slice(frame);
            self.peak_level = level.smoothed_rms;
        } else if level.is_speaking {
            self.phrase.extend_from_slice(frame);
            self.peak_level = self.peak_level.max(level.smoothed_rms);
            if self.phrase.len() >= self.seconds_to_samples(MAX_PHRASE_SECONDS) {
                self.detector.reset();
                phrase = Some(self.take_phrase(PhraseEnd::MaxLength));
            }
        } else if level.speech_just_ended {
            self.phrase.extend_from_slice(frame);
            if self.phrase.len() >= self.seconds_to_samples(MIN_PHRASE_SECONDS) {
                phrase = Some(self.take_phrase(PhraseEnd::SpeechStopped));
            } else {
                self.phrase.clear();
                self.peak_level = 0.0;
            }
        }

        if self.lead_in.len() >= LEAD_IN_FRAMES {
            self.lead_in.pop_front();
        }
        self.lead_in.push_back(frame.to_vec());

        FrameOutcome { level, phrase }
    }

    /// Drops any phrase in progress, e.g. while listening is paused.
    pub fn reset(&mut self) {
        self.detector.reset();
        self.phrase.clear();
        self.peak_level = 0.0;
    }

    fn seconds_to_samples(&self, seconds: f32) -> usize {
        (self.sample_rate as f32 * seconds) as usize
    }

    fn take_phrase(&mut self, ended_by: PhraseEnd) -> SpeechPhrase {
        let samples = std::mem::take(&mut self.phrase);
        let peak_level = std::mem::replace(&mut self.peak_level, 0.0);
        SpeechPhrase {
            duration_secs: samples.len() as f32 / self.sample_rate as f32,
            samples,
            peak_level,
            ended_by,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16_000;
    const THRESHOLD: f32 = 0.035;

    fn feed(segmenter: &mut SpeechSegmenter, seconds: f32, amplitude: f32) -> Vec<SpeechPhrase> {
        let mut phrases = Vec::new();
        let count = (RATE as f32 * seconds) as usize;
        for index in 0..count {
            let sample = amplitude * if index % 2 == 0 { 1.0 } else { -1.0 };
            if let Some(frame) = segmenter.push_sample(sample) {
                phrases.extend(segmenter.process_frame(&frame, THRESHOLD).phrase);
            }
        }
        phrases
    }

    #[test]
    fn speech_followed_by_silence_is_one_phrase_with_lead_in() {
        let mut segmenter = SpeechSegmenter::new(RATE);
        feed(&mut segmenter, 0.5, 0.0);
        feed(&mut segmenter, 1.0, 0.3);
        let phrases = feed(&mut segmenter, 1.5, 0.0);
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0].ended_by, PhraseEnd::SpeechStopped);
        assert!(phrases[0].duration_secs > 1.0, "lead-in and tail are kept");
    }

    #[test]
    fn long_speech_is_cut_at_the_maximum_length() {
        let mut segmenter = SpeechSegmenter::new(RATE);
        let phrases = feed(&mut segmenter, 5.0, 0.3);
        assert_eq!(phrases[0].ended_by, PhraseEnd::MaxLength);
        assert!(phrases[0].duration_secs <= MAX_PHRASE_SECONDS + FRAME_SECONDS * 2.0);
    }

    #[test]
    fn silence_produces_no_phrase() {
        let mut segmenter = SpeechSegmenter::new(RATE);
        assert!(feed(&mut segmenter, 2.0, 0.001).is_empty());
    }
}
