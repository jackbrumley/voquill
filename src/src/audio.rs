use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub async fn record_audio_while_flag(is_recording: &Arc<Mutex<bool>>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    
    // On Linux, try to find a shared device (PulseAudio/PipeWire) first to avoid "Device Busy" errors
    #[cfg(target_os = "linux")]
    let device = {
        let devices = host.input_devices()?;
        let mut target_device = None;
        for dev in devices {
            if let Ok(name) = dev.name() {
                println!("Checking audio device: {}", name);
                // Prioritize virtual/shared devices
                if name.to_lowercase().contains("pulse") || name.to_lowercase().contains("pipewire") || name.to_lowercase().contains("default") {
                    target_device = Some(dev);
                    break;
                }
            }
        }
        target_device.or_else(|| host.default_input_device())
    };

    #[cfg(not(target_os = "linux"))]
    let device = host.default_input_device();

    let device = device.ok_or("No input device available")?;
    println!("🎤 Using audio device: {}", device.name().unwrap_or_else(|_| "Unknown".into()));

    let default_config = device.default_input_config()?;
    
    // Use device's native settings for recording - this is guaranteed to work
    let config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    
    log::info!("Recording with device native settings: {}Hz, {} channels", 
               config.sample_rate.0,
               config.channels);
    
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

    // Convert audio to optimal format for Whisper (16kHz mono)
    let optimized_audio = convert_audio_for_whisper(&audio_data, config.sample_rate.0, config.channels)?;
    
    Ok(optimized_audio)
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

fn convert_audio_for_whisper(
    audio_data: &[u8], 
    original_sample_rate: u32, 
    original_channels: u16
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // If already optimal format, return as-is
    if original_sample_rate == 16000 && original_channels == 1 {
        log::info!("Audio already in optimal format (16kHz mono), no conversion needed");
        return Ok(audio_data.to_vec());
    }
    
    log::info!("Converting audio from {}Hz {} channels to 16kHz mono for optimal Whisper performance", 
               original_sample_rate, original_channels);
    
    // Parse the original WAV file
    let mut cursor = std::io::Cursor::new(audio_data);
    let mut reader = hound::WavReader::new(&mut cursor)?;
    
    let spec = reader.spec();
    let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let samples = samples?;
    
    // Convert to mono if stereo
    let mono_samples = if spec.channels == 2 {
        // Convert stereo to mono by averaging left and right channels
        samples.chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16
                } else {
                    chunk[0]
                }
            })
            .collect()
    } else {
        samples
    };
    
    // Resample to 16kHz if needed
    let resampled_samples = if spec.sample_rate != 16000 {
        resample_audio(&mono_samples, spec.sample_rate, 16000)
    } else {
        mono_samples
    };
    
    // Create new WAV file with optimal settings
    let optimized_spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut output_buffer = Vec::new();
    {
        let mut writer = WavWriter::new(std::io::Cursor::new(&mut output_buffer), optimized_spec)?;
        for sample in resampled_samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }
    
    let original_size = audio_data.len();
    let optimized_size = output_buffer.len();
    let reduction_percent = ((original_size as f64 - optimized_size as f64) / original_size as f64) * 100.0;
    
    log::info!("Audio conversion complete: {} bytes -> {} bytes ({:.1}% reduction)", 
               original_size, optimized_size, reduction_percent);
    
    Ok(output_buffer)
}

fn resample_audio(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    
    for i in 0..output_len {
        let src_index = (i as f64 * ratio) as usize;
        if src_index < samples.len() {
            output.push(samples[src_index]);
        }
    }
    
    output
}
