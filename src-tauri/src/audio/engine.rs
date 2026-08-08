use std::sync::{mpsc, Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use ringbuf::traits::*;
use ringbuf::{CachingCons, HeapRb};

pub struct PersistentAudioEngine {
    pub stream: cpal::Stream,
    pub pre_roll_consumer: Arc<Mutex<CachingCons<Arc<HeapRb<f32>>>>>,
    pub recording_tx: Arc<Mutex<Option<mpsc::SyncSender<f32>>>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl PersistentAudioEngine {
    pub fn new(device: &cpal::Device, sensitivity: f32) -> Result<Self, String> {
        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        crate::log_info!(
            "Audio Engine: Opening native stream ({}Hz, {} channels)",
            sample_rate,
            channels
        );

        let pre_roll_size = (sample_rate as f32 * 0.2) as usize;
        let pre_roll_rb = HeapRb::<f32>::new(pre_roll_size);
        let (mut pre_roll_prod, pre_roll_cons) = pre_roll_rb.split();

        let recording_tx = Arc::new(Mutex::new(None::<mpsc::SyncSender<f32>>));
        let recording_tx_clone = recording_tx.clone();

        let err_fn = |err| crate::log_info!("Audio stream error: {}", err);

        let stream_config: cpal::StreamConfig = config.clone().into();
        let channels_usize = channels as usize;

        let mut audio_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for frame in data.chunks(channels_usize) {
                let sample_raw: f32 = frame.iter().sum();
                let sample = sample_raw * sensitivity;

                let _ = pre_roll_prod.try_push(sample);
                if let Ok(guard) = recording_tx_clone.try_lock() {
                    if let Some(tx) = guard.as_ref() {
                        let _ = tx.try_send(sample);
                    }
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
            sample_rate,
            channels,
        })
    }
}
