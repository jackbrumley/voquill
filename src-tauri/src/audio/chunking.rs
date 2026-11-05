//! Splits long recordings into pieces a transcription provider can accept.
//!
//! Providers have hard limits on one request (the Parakeet sidecar rejects
//! utterances over 300 s, the OpenAI API rejects uploads over 25 MB). Each
//! provider declares its own maximum piece length and calls
//! `split_at_pauses`, which cuts at the quietest moment near each limit so a
//! word is rarely cut in half.

use std::ops::Range;

/// How far back from a piece's maximum length to look for a pause to cut at.
const SEARCH_WINDOW_SECONDS: f32 = 10.0;

/// Length of the frames compared when looking for the quietest cut point.
const FRAME_SECONDS: f32 = 0.1;

/// Splits `samples` into consecutive ranges no longer than `max_seconds`,
/// covering every sample exactly once. Audio that already fits is returned
/// as a single range. Each cut is placed at the start of the quietest frame
/// in the last `SEARCH_WINDOW_SECONDS` (or the last half, for short limits)
/// before the limit.
pub fn split_at_pauses<T: Copy + Into<f32>>(
    samples: &[T],
    sample_rate: u32,
    max_seconds: f32,
) -> Vec<Range<usize>> {
    let max_length = ((max_seconds * sample_rate as f32) as usize).max(1);
    let frame_length = ((FRAME_SECONDS * sample_rate as f32) as usize).max(1);
    let search_length = ((SEARCH_WINDOW_SECONDS.min(max_seconds / 2.0) * sample_rate as f32)
        as usize)
        .clamp(1, max_length);

    let mut pieces = Vec::new();
    let mut start = 0;
    while samples.len() - start > max_length {
        let window_end = start + max_length;
        let window_start = window_end - search_length;
        let cut =
            window_start + quietest_frame_offset(&samples[window_start..window_end], frame_length);
        // The window never starts at `start` for real limits, but guard
        // against an empty piece so the loop always advances.
        let cut = if cut > start { cut } else { window_end };
        pieces.push(start..cut);
        start = cut;
    }
    pieces.push(start..samples.len());
    pieces
}

/// Offset of the frame in `window` with the least energy.
fn quietest_frame_offset<T: Copy + Into<f32>>(window: &[T], frame_length: usize) -> usize {
    window
        .chunks(frame_length)
        .enumerate()
        .map(|(frame_index, frame)| (frame_index * frame_length, frame_energy(frame)))
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(offset, _)| offset)
        .unwrap_or(0)
}

fn frame_energy<T: Copy + Into<f32>>(frame: &[T]) -> f32 {
    let sum_of_squares: f32 = frame
        .iter()
        .map(|&sample| {
            let value: f32 = sample.into();
            value * value
        })
        .sum();
    sum_of_squares / frame.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 1000;

    fn tone(seconds: usize) -> Vec<f32> {
        vec![0.5; seconds * RATE as usize]
    }

    fn assert_covers(pieces: &[Range<usize>], length: usize) {
        assert_eq!(pieces.first().unwrap().start, 0);
        assert_eq!(pieces.last().unwrap().end, length);
        for pair in pieces.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
    }

    #[test]
    fn short_audio_is_one_piece() {
        let samples = tone(30);
        assert_eq!(
            split_at_pauses(&samples, RATE, 60.0),
            vec![0..samples.len()]
        );
    }

    #[test]
    fn empty_audio_is_one_empty_piece() {
        let samples: Vec<f32> = Vec::new();
        assert_eq!(split_at_pauses(&samples, RATE, 60.0), vec![0..0]);
    }

    #[test]
    fn long_audio_pieces_respect_the_limit_and_cover_everything() {
        let samples = tone(1001);
        let pieces = split_at_pauses(&samples, RATE, 60.0);
        assert!(pieces.len() >= 17);
        assert!(pieces.iter().all(|piece| piece.len() <= 60 * RATE as usize));
        assert_covers(&pieces, samples.len());
    }

    #[test]
    fn cuts_at_the_pause_before_the_limit() {
        let mut samples = tone(90);
        let pause_start = 55 * RATE as usize;
        samples[pause_start..pause_start + 200].fill(0.0);
        let pieces = split_at_pauses(&samples, RATE, 60.0);
        assert_eq!(pieces[0], 0..pause_start);
        assert_covers(&pieces, samples.len());
    }

    #[test]
    fn works_on_integer_samples() {
        let samples: Vec<i16> = vec![1000; 130 * RATE as usize];
        let pieces = split_at_pauses(&samples, RATE, 60.0);
        assert_eq!(pieces.len(), 3);
        assert_covers(&pieces, samples.len());
    }
}
