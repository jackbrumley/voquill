use std::sync::{mpsc, Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use ringbuf::traits::*;
use ringbuf::HeapRb;

/// Always-on mono capture stream feeding the pre-roll buffer, dictation and
/// the voice macro listener. Created and replaced only by
/// `app::capture::ensure_capture_engine`.
pub struct PersistentAudioEngine {
    _stream: cpal::Stream,
    /// The most recent ~200 ms of audio, oldest samples overwritten first, so
    /// a dictation keeps the speech that began just before the hotkey press.
    pre_roll: Arc<Mutex<HeapRb<f32>>>,
    pub recording_tx: Arc<Mutex<Option<mpsc::SyncSender<f32>>>>,
    pub macro_tx: Arc<Mutex<Option<mpsc::SyncSender<f32>>>>,
    pub sample_rate: u32,
    /// Identity of the endpoint the stream was opened on.
    pub endpoint_identity: String,
    /// Input gain baked into the capture callback.
    pub sensitivity: f32,
    stream_failure: Arc<Mutex<Option<String>>>,
}

impl PersistentAudioEngine {
    /// Sends every sample captured from now on to `recording_tx` and returns
    /// the pre-roll captured just before, so the two join without a gap.
    pub fn begin_recording(&self, recording_tx: mpsc::SyncSender<f32>) -> Vec<f32> {
        let mut pre_roll = self.pre_roll.lock().unwrap();
        let samples: Vec<f32> = pre_roll.pop_iter().collect();
        *self.recording_tx.lock().unwrap() = Some(recording_tx);
        samples
    }

    pub fn new(endpoint: &super::CaptureEndpoint, sensitivity: f32) -> Result<Self, String> {
        let device = &endpoint.device;
        let candidates = candidate_capture_configs(device);
        if candidates.is_empty() {
            return Err("Input device offers no F32 or I16 capture configuration".to_string());
        }

        let mut failures = Vec::new();
        for config in candidates {
            match Self::try_build_stream(endpoint, config, sensitivity) {
                Ok(engine) => return Ok(engine),
                Err(error) => {
                    crate::log_warn!(
                        "Audio Engine: Capture config ({}Hz, {} channels, {:?}) failed: {}",
                        config.sample_rate(),
                        config.channels(),
                        config.sample_format(),
                        error
                    );
                    failures.push(error);
                }
            }
        }
        Err(format!(
            "Failed to open input stream with any supported config: {}",
            failures.join("; ")
        ))
    }

    /// The fatal error the stream reported, if any. A failed engine captures
    /// nothing and must be rebuilt.
    pub fn stream_failure(&self) -> Option<String> {
        self.stream_failure.lock().unwrap().clone()
    }

    fn try_build_stream(
        endpoint: &super::CaptureEndpoint,
        config: cpal::SupportedStreamConfig,
        sensitivity: f32,
    ) -> Result<Self, String> {
        let device = &endpoint.device;
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        crate::log_info!(
            "Audio Engine: Opening stream on {} ({}Hz, {} channels, format={:?})",
            endpoint.identity,
            sample_rate,
            channels,
            config.sample_format()
        );

        let pre_roll_size = (sample_rate as f32 * 0.2) as usize;
        let pre_roll = Arc::new(Mutex::new(HeapRb::<f32>::new(pre_roll_size)));
        let pre_roll_clone = pre_roll.clone();

        let recording_tx = Arc::new(Mutex::new(None::<mpsc::SyncSender<f32>>));
        let recording_tx_clone = recording_tx.clone();

        let macro_tx = Arc::new(Mutex::new(None::<mpsc::SyncSender<f32>>));
        let macro_tx_clone = macro_tx.clone();

        let stream_failure = Arc::new(Mutex::new(None::<String>));
        let stream_failure_clone = stream_failure.clone();
        let err_fn = move |error: cpal::Error| {
            if is_fatal_stream_error(error.kind()) {
                crate::log_warn!("Audio Engine: Capture stream failed: {}", error);
                let mut failure = stream_failure_clone.lock().unwrap();
                if failure.is_none() {
                    *failure = Some(error.to_string());
                }
            } else {
                crate::log_info!("Audio Engine: Capture stream notice: {}", error);
            }
        };

        let mut stream_config = config.config();
        stream_config.buffer_size = super::device::stream_buffer_size(device, &config);
        let channels_usize = channels as usize;

        let audio_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Lock pre_roll, recording_tx and macro_tx ONCE per callback buffer
            // (~512 samples / ~11ms) instead of per-sample, to eliminate lock
            // contention on the audio thread. pre_roll is locked first, matching
            // `begin_recording`, so a buffer lands either in the pre-roll it
            // drains or in the recording it arms.
            let mut pre_roll_guard = pre_roll_clone.try_lock();
            let guard = recording_tx_clone.try_lock();
            let recording_tx_ref = guard.as_ref().ok().and_then(|g| g.as_ref());

            let macro_guard = macro_tx_clone.try_lock();
            let macro_tx_ref = macro_guard.as_ref().ok().and_then(|g| g.as_ref());

            for frame in data.chunks(channels_usize) {
                let sample_raw: f32 = frame.iter().sum::<f32>() / channels_usize as f32;
                let sample = sample_raw * sensitivity;

                if let Ok(pre_roll) = pre_roll_guard.as_mut() {
                    pre_roll.push_overwrite(sample);
                }
                if let Some(tx) = recording_tx_ref {
                    let _ = tx.try_send(sample);
                }
                if let Some(tx) = macro_tx_ref {
                    let _ = tx.try_send(sample);
                }
            }
        };

        let stream = match config.sample_format() {
            SampleFormat::F32 => {
                device.build_input_stream(stream_config, audio_callback, err_fn, None)
            }
            SampleFormat::I16 => device.build_input_stream(
                stream_config,
                move |data: &[i16], info| {
                    let f32_data: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    audio_callback(&f32_data, info);
                },
                err_fn,
                None,
            ),
            _ => return Err("Unsupported sample format".into()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        Ok(Self {
            _stream: stream,
            pre_roll,
            recording_tx,
            macro_tx,
            sample_rate,
            endpoint_identity: endpoint.identity.clone(),
            sensitivity,
            stream_failure,
        })
    }
}

/// Whether a stream error leaves the capture stream unusable. Xruns, refused
/// real-time scheduling and transparent reroutes are reported but the stream
/// keeps delivering audio.
fn is_fatal_stream_error(kind: cpal::ErrorKind) -> bool {
    !matches!(
        kind,
        cpal::ErrorKind::Xrun | cpal::ErrorKind::RealtimeDenied | cpal::ErrorKind::DeviceChanged
    )
}

/// Sample formats the capture callback understands, in preference order.
const CAPTURE_SAMPLE_FORMATS: [SampleFormat; 2] = [SampleFormat::F32, SampleFormat::I16];

/// Whisper's native rate; capturing at it avoids a resampling pass later.
const TARGET_SAMPLE_RATE: cpal::SampleRate = 16000;

/// Capture configs to try, best first: 16kHz mono, then 16kHz with any
/// channel count (downmixed in the callback), then the device's own default
/// rate and channel layout. Only formats the callback converts are offered,
/// because cpal's default config may report I24/I32 on high-precision devices.
fn candidate_capture_configs(device: &cpal::Device) -> Vec<cpal::SupportedStreamConfig> {
    let ranges: Vec<cpal::SupportedStreamConfigRange> = match device.supported_input_configs() {
        Ok(ranges) => ranges.collect(),
        Err(error) => {
            crate::log_warn!("Audio Engine: Could not query input configs: {}", error);
            Vec::new()
        }
    };
    let default_config = device.default_input_config().ok();

    let mut candidates = Vec::new();
    let mut push_candidate = |config: cpal::SupportedStreamConfig| {
        if !candidates.contains(&config) {
            candidates.push(config);
        }
    };

    for format in CAPTURE_SAMPLE_FORMATS {
        for range in ranges.iter().filter(|range| {
            range.sample_format() == format
                && range.channels() == 1
                && range.contains_rate(TARGET_SAMPLE_RATE)
        }) {
            push_candidate(range.with_sample_rate(TARGET_SAMPLE_RATE));
        }
    }

    for format in CAPTURE_SAMPLE_FORMATS {
        for range in ranges.iter().filter(|range| {
            range.sample_format() == format && range.contains_rate(TARGET_SAMPLE_RATE)
        }) {
            push_candidate(range.with_sample_rate(TARGET_SAMPLE_RATE));
        }
    }

    if let Some(default_config) = default_config {
        if CAPTURE_SAMPLE_FORMATS.contains(&default_config.sample_format()) {
            push_candidate(default_config);
        }
        for format in CAPTURE_SAMPLE_FORMATS {
            for range in ranges.iter().filter(|range| {
                range.sample_format() == format
                    && range.channels() == default_config.channels()
                    && range.contains_rate(default_config.sample_rate())
            }) {
                push_candidate(range.with_sample_rate(default_config.sample_rate()));
            }
        }
    }

    candidates
}
