use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use super::conversion::resample_audio_f32;

pub fn play_audio<F>(
    samples: Vec<f32>,
    sample_rate: u32,
    on_done: F,
) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() + Send + 'static,
{
    let host = cpal::default_host();
    let device = {
        let mut selected = None;
        if let Ok(devices) = host.output_devices() {
            for dev in devices {
                if let Ok(id) = dev.id() {
                    #[cfg(target_os = "linux")]
                    if id.1 == "pulse" || id.1.starts_with("default") {
                        selected = Some(dev);
                        break;
                    }
                    #[cfg(not(target_os = "linux"))]
                    if id.1.starts_with("default") {
                        selected = Some(dev);
                        break;
                    }
                }
            }
        }
        selected.or_else(|| host.default_output_device())
    }
    .ok_or("No output device available")?;

    let config = device.default_output_config()?;
    let stream_config: StreamConfig = config.clone().into();
    let resampled = Arc::new(resample_audio_f32(
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
