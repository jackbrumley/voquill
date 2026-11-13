//! Lowers or mutes other applications' audio while a dictation records, and
//! restores it afterwards (issue #18).
//!
//! This module is the single owner of that adjustment: the recording flow
//! calls `adjust_for_recording` when capture starts and `restore` when it
//! ends, and the quit path calls `restore` so the app never exits leaving
//! other audio lowered. Restoring only undoes what Voquill changed: a stream
//! the user muted beforehand stays muted, and a stream whose volume or mute
//! the user changed during the recording is left as the user set it.
//! Voquill's own streams (voice macro chimes, TTS) are never touched.
//!
//! Each OS backend is a module with the same three items: `AdjustedStreams`,
//! `adjust` and `restore`.

use crate::config::OtherAudioWhileRecording;
use std::sync::Mutex;

#[cfg(target_os = "linux")]
mod pulse;
#[cfg(target_os = "linux")]
use pulse as backend;

#[cfg(target_os = "windows")]
mod wasapi;
#[cfg(target_os = "windows")]
use wasapi as backend;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use unsupported as backend;

/// What happens to other applications' audio during a recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adjustment {
    /// Scale each stream to `LOWERED_VOLUME_FACTOR` of its volume.
    Lower,
    Mute,
}

/// Linear amplitude factor applied by `Adjustment::Lower` (about -14 dB):
/// clearly in the background, still audible. Only the backends that can
/// change volume use it.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub const LOWERED_VOLUME_FACTOR: f32 = 0.2;

/// The streams the current recording adjusted, waiting to be restored.
static ADJUSTED_STREAMS: Mutex<Option<backend::AdjustedStreams>> = Mutex::new(None);

/// Whether this platform can adjust other applications' audio. macOS has no
/// public per-application volume API.
pub fn is_supported() -> bool {
    cfg!(any(target_os = "linux", target_os = "windows"))
}

impl OtherAudioWhileRecording {
    fn adjustment(self) -> Option<Adjustment> {
        match self {
            OtherAudioWhileRecording::Keep => None,
            OtherAudioWhileRecording::Lower => Some(Adjustment::Lower),
            OtherAudioWhileRecording::Mute => Some(Adjustment::Mute),
        }
    }
}

/// Lowers or mutes other applications' audio as `setting` asks. Blocking
/// (it talks to the audio server); run it off the async runtime.
pub fn adjust_for_recording(setting: OtherAudioWhileRecording) {
    let Some(adjustment) = setting.adjustment() else {
        return;
    };
    // A previous adjustment that was never restored would otherwise be lost.
    restore();

    match backend::adjust(adjustment) {
        Ok(streams) => {
            crate::log_info!(
                "Other audio while recording: {:?} applied to {} stream(s)",
                adjustment,
                streams.len()
            );
            *ADJUSTED_STREAMS.lock().unwrap() = Some(streams);
        }
        Err(error) => {
            crate::log_warn!(
                "Other audio while recording: could not {:?} other audio: {}",
                adjustment,
                error
            );
        }
    }
}

/// Undoes the last `adjust_for_recording`, if any. Blocking; safe to call
/// when nothing is adjusted.
pub fn restore() {
    let Some(streams) = ADJUSTED_STREAMS.lock().unwrap().take() else {
        return;
    };
    let count = streams.len();
    match backend::restore(streams) {
        Ok(()) => crate::log_info!("Other audio while recording: restored {} stream(s)", count),
        Err(error) => crate::log_warn!(
            "Other audio while recording: could not restore other audio: {}",
            error
        ),
    }
}
