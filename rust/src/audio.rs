use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub async fn record_audio_while_flag(is_recording: &Arc<Mutex<bool>>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    let default_config = device.default_input_config()?;
    let config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    
    // Create temporary file for recording
    let temp_path = std::env::temp_dir().join("voquill_recording.wav");
    let spec = WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate.0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = WavWriter::create(&temp_path, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_2 = writer.clone();

    let err_fn = |err| {
        log::error!("Audio stream error: {}", err);
    };

    // Build stream based on the device's default sample format
    let stream = match default_config.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                write_input_data_f32(data, &writer_2)
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                write_input_data_i16(data, &writer_2)
            },
            err_fn,
            None,
        )?,
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                write_input_data_u16(data, &writer_2)
            },
            err_fn,
            None,
        )?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    // Record while the flag is true
    while {
        let recording = is_recording.lock().unwrap();
        *recording
    } {
        std::thread::sleep(Duration::from_millis(10));
    }

    drop(stream);

    // Finalize the WAV file
    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize()?;
        }
    }

    // Read the recorded file
    let audio_data = std::fs::read(&temp_path)?;
    
    // Clean up temporary file
    let _ = std::fs::remove_file(&temp_path);

    Ok(audio_data)
}

pub async fn record_audio(duration_seconds: f32) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    let default_config = device.default_input_config()?;
    let config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    
    // Create temporary file for recording
    let temp_path = std::env::temp_dir().join("voquill_recording.wav");
    let spec = WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate.0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = WavWriter::create(&temp_path, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_2 = writer.clone();

    let err_fn = |err| {
        log::error!("Audio stream error: {}", err);
    };

    // Build stream based on the device's default sample format
    let stream = match default_config.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                write_input_data_f32(data, &writer_2)
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                write_input_data_i16(data, &writer_2)
            },
            err_fn,
            None,
        )?,
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                write_input_data_u16(data, &writer_2)
            },
            err_fn,
            None,
        )?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;

    // Record for the specified duration using std::thread::sleep instead of tokio::sleep
    // to avoid Send trait issues
    std::thread::sleep(Duration::from_secs_f32(duration_seconds));

    drop(stream);

    // Finalize the WAV file
    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize()?;
        }
    }

    // Read the recorded file
    let audio_data = std::fs::read(&temp_path)?;
    
    // Clean up temporary file
    let _ = std::fs::remove_file(&temp_path);

    Ok(audio_data)
}

fn get_supported_config(device: &Device) -> Result<StreamConfig, Box<dyn std::error::Error + Send + Sync>> {
    // Simply use the device's default configuration - this is what most audio apps do
    let default_config = device.default_input_config()?;
    
    log::info!("Using device default config: channels={}, sample_rate={}, sample_format={:?}", 
               default_config.channels(), 
               default_config.sample_rate().0, 
               default_config.sample_format());
    
    Ok(StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    })
}

fn write_input_data_f32(input: &[f32], writer: &Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>) {
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                // Convert f32 to i16 for WAV file
                let sample_i16 = (sample * i16::MAX as f32) as i16;
                let _ = writer.write_sample(sample_i16);
            }
        }
    }
}

fn write_input_data_i16(input: &[i16], writer: &Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>) {
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let _ = writer.write_sample(sample);
            }
        }
    }
}

fn write_input_data_u16(input: &[u16], writer: &Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>) {
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                // Convert u16 to i16 for WAV file
                let sample_i16 = (sample as i32 - 32768) as i16;
                let _ = writer.write_sample(sample_i16);
            }
        }
    }
}
