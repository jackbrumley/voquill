use std::sync::Arc;

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use super::conversion::resample_linear;

pub fn play_audio_on_device<F>(
    device: &cpal::Device,
    samples: Vec<f32>,
    sample_rate: u32,
    on_done: F,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() + Send + 'static,
{
    let config = device.default_output_config()?;
    let stream_config: StreamConfig = config.clone().into();
    let resampled = Arc::new(resample_linear(
        &samples,
        sample_rate,
        stream_config.sample_rate,
    ));
    let chans = stream_config.channels as usize;

    let err_fn = |err| crate::log_info!("Playback error: {}", err);
    let mut done = Some(on_done);

    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            let resampled_clone = resampled.clone();
            let mut idx = 0;
            device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in data.chunks_mut(chans) {
                        if idx < resampled_clone.len() {
                            let s = resampled_clone[idx];
                            for out in frame.iter_mut() {
                                *out = s;
                            }
                            idx += 1;
                        } else {
                            for out in frame.iter_mut() {
                                *out = 0.0;
                            }
                            if let Some(cb) = done.take() {
                                cb();
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            let resampled_clone = resampled.clone();
            let mut idx = 0;
            device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _| {
                    for frame in data.chunks_mut(chans) {
                        if idx < resampled_clone.len() {
                            let s = (resampled_clone[idx] * i16::MAX as f32) as i16;
                            for out in frame.iter_mut() {
                                *out = s;
                            }
                            idx += 1;
                        } else {
                            for out in frame.iter_mut() {
                                *out = 0;
                            }
                            if let Some(cb) = done.take() {
                                cb();
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => return Err("Unsupported format".into()),
    };
    stream.play()?;
    Ok(stream)
}

pub fn play_audio<F>(
    samples: Vec<f32>,
    sample_rate: u32,
    device_id: Option<String>,
    on_done: F,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() + Send + 'static,
{
    let device = super::device::lookup_output_device(device_id)?;
    play_audio_on_device(&device, samples, sample_rate, on_done)
}

pub fn play_wav_file<F>(
    wav_bytes: &[u8],
    device_id: Option<String>,
    on_done: F,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() + Send + 'static,
{
    let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes))?;
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| (s as f32 / max_val).clamp(-1.0, 1.0))
                .collect()
        }
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(|s| s.ok()).collect(),
    };
    let mono_samples = if spec.channels > 1 {
        let chans = spec.channels as usize;
        samples
            .chunks_exact(chans)
            .map(|frame| frame.iter().sum::<f32>() / chans as f32)
            .collect()
    } else {
        samples
    };
    play_audio(mono_samples, spec.sample_rate, device_id, on_done)
}
