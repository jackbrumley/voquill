use std::sync::{mpsc, Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use ringbuf::traits::*;
use ringbuf::{CachingCons, HeapRb};

pub struct PersistentAudioEngine {
    pub stream: cpal::Stream,
    pub pre_roll_consumer: Arc<Mutex<CachingCons<Arc<HeapRb<f32>>>>>,
    pub recording_tx: Arc<Mutex<Option<mpsc::SyncSender<f32>>>>,
    pub macro_tx: Arc<Mutex<Option<mpsc::SyncSender<f32>>>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl PersistentAudioEngine {
    pub fn new(device: &cpal::Device, sensitivity: f32) -> Result<Self, String> {
        let preferred_config = select_preferred_config(device);
        match Self::try_build_stream(device, preferred_config, sensitivity) {
            Ok(engine) => Ok(engine),
            Err(err) => {
                crate::log_warn!(
                    "Audio Engine: Preferred stream init failed ({}); falling back to default config",
                    err
                );
                let default_config = device
                    .default_input_config()
                    .map_err(|e| format!("Failed to get default input config: {}", e))?;
                Self::try_build_stream(device, default_config, sensitivity)
            }
        }
    }

    fn try_build_stream(
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        sensitivity: f32,
    ) -> Result<Self, String> {
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        crate::log_info!(
            "Audio Engine: Opening stream ({}Hz, {} channels, format={:?})",
            sample_rate,
            channels,
            config.sample_format()
        );

        let pre_roll_size = (sample_rate as f32 * 0.2) as usize;
        let pre_roll_rb = HeapRb::<f32>::new(pre_roll_size);
        let (mut pre_roll_prod, pre_roll_cons) = pre_roll_rb.split();

        let recording_tx = Arc::new(Mutex::new(None::<mpsc::SyncSender<f32>>));
        let recording_tx_clone = recording_tx.clone();

        let macro_tx = Arc::new(Mutex::new(None::<mpsc::SyncSender<f32>>));
        let macro_tx_clone = macro_tx.clone();

        let err_fn = |err| crate::log_info!("Audio stream error: {}", err);

        let stream_config: cpal::StreamConfig = config.clone().into();
        let channels_usize = channels as usize;

        let mut audio_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Lock recording_tx and macro_tx ONCE per callback buffer (~512 samples / ~11ms)
            // instead of per-sample, to eliminate lock contention on the audio thread.
            let guard = recording_tx_clone.try_lock();
            let recording_tx_ref = guard.as_ref().ok().and_then(|g| g.as_ref());

            let macro_guard = macro_tx_clone.try_lock();
            let macro_tx_ref = macro_guard.as_ref().ok().and_then(|g| g.as_ref());

            for frame in data.chunks(channels_usize) {
                let sample_raw: f32 = frame.iter().sum::<f32>() / channels_usize as f32;
                let sample = sample_raw * sensitivity;

                let _ = pre_roll_prod.try_push(sample);
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
                device.build_input_stream(&stream_config, audio_callback, err_fn, None)
            }
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
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
            stream,
            pre_roll_consumer: Arc::new(Mutex::new(pre_roll_cons)),
            recording_tx,
            macro_tx,
            sample_rate,
            channels: 1, // downmixed to mono in callback
        })
    }
}

fn select_preferred_config(device: &cpal::Device) -> cpal::SupportedStreamConfig {
    let target_rate: cpal::SampleRate = 16000;

    if let Ok(ranges) = device.supported_input_configs() {
        let ranges_vec: Vec<_> = ranges.collect();

        // 1. First choice: 16kHz mono (1 channel)
        for range in &ranges_vec {
            if range.channels() == 1
                && range.min_sample_rate() <= target_rate
                && target_rate <= range.max_sample_rate()
            {
                crate::log_info!("Audio Engine: Probed device supports 16000Hz mono");
                return range.with_sample_rate(target_rate);
            }
        }

        // 2. Second choice: 16kHz any channels (will be downmixed to mono in callback)
        for range in &ranges_vec {
            if range.min_sample_rate() <= target_rate && target_rate <= range.max_sample_rate() {
                crate::log_info!(
                    "Audio Engine: Probed device supports 16000Hz ({} channels)",
                    range.channels()
                );
                return range.with_sample_rate(target_rate);
            }
        }
    }

    // 3. Fall back to device default
    device.default_input_config().unwrap_or_else(|_| {
        cpal::SupportedStreamConfig::new(
            1,
            target_rate,
            cpal::SupportedBufferSize::Unknown,
            SampleFormat::F32,
        )
    })
}
