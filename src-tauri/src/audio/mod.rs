pub mod chunking;
pub mod conversion;
pub mod decode;
pub mod device;
pub mod engine;
pub mod playback;
#[cfg(target_os = "linux")]
mod pulse_server;
pub mod recording;
pub mod vad;

pub use conversion::{convert_audio_file_for_whisper, extract_segment_wav};
pub use device::{
    get_input_devices, get_output_devices, resolve_capture_endpoint, AudioDevice, CaptureEndpoint,
};
pub use engine::PersistentAudioEngine;
pub use playback::{AudioClip, AudioPlayer, PlaybackSlot};
pub use recording::{record_audio_while_flag, record_mic_test};
