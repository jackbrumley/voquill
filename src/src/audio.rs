use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(target_os = "linux")]
use pulsectl::controllers::DeviceControl;

#[derive(serde::Serialize, Clone, Debug)]
pub struct AudioDevice {
    pub id: String,
    pub label: String,
}

#[cfg(target_os = "linux")]
fn get_linux_pulse_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    let mut handler = match pulsectl::controllers::SourceController::create() {
        Ok(h) => h,
        Err(e) => return Err(format!("Failed to connect to PulseAudio: {}", e)),
    };

    let sources = match handler.list_devices() {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to list PulseAudio sources: {}", e)),
    };

    for source in sources {
        let name = source.name.clone().unwrap_or_default();
        let description = source.description.clone().unwrap_or_default();

        let lower_name = name.to_lowercase();
        let lower_desc = description.to_lowercase();
        
        if lower_name.contains(".monitor") || lower_desc.contains("monitor") || lower_name == "null" {
            continue;
        }

        devices.push(AudioDevice {
            id: format!("pulse:{}", name),
            label: description,
        });
    }

    Ok(devices)
}

pub fn get_input_devices() -> Result<Vec<AudioDevice>, String> {
    println!("🔍 get_input_devices called from frontend");
    
    let mut final_devices = Vec::new();

    #[cfg(target_os = "linux")]
    {
        match get_linux_pulse_devices() {
            Ok(devices) => {
                println!("PulseAudio discovery found {} devices", devices.len());
                final_devices = devices;
            }
            Err(e) => {
                println!("⚠️ PulseAudio discovery failed, falling back to ALSA: {}", e);
            }
        }
    }

    if final_devices.is_empty() {
        let mut seen_labels = std::collections::HashSet::new();
        let available_hosts = cpal::available_hosts();
        for host_id in available_hosts {
            if let Ok(host) = cpal::host_from_id(host_id) {
                if let Ok(devices) = host.input_devices() {
                    for dev in devices {
                        let device_id = match dev.id() {
                            Ok(id) => id.1,
                            Err(_) => continue,
                        };

                        if !device_id.starts_with("default:") && device_id != "pulse" && device_id != "default" {
                            continue;
                        }

                        let label = match dev.description() {
                            Ok(desc) => desc.name().to_string(),
                            Err(_) => device_id.clone(),
                        };

                        let clean_label = label
                            .split(", USB Audio").next().unwrap_or(&label)
                            .split(", ALC").next().unwrap_or(&label)
                            .trim()
                            .to_string();

                        if !seen_labels.contains(&clean_label) {
                            final_devices.push(AudioDevice {
                                id: device_id.clone(),
                                label: clean_label.clone(),
                            });
                            seen_labels.insert(clean_label);
                        }
                    }
                }
            }
        }
    }

    final_devices.sort_by(|a, b| a.label.cmp(&b.label));
    
    // Prepend "System Default" so it's always the first option and selectable
    final_devices.insert(0, AudioDevice {
        id: "default".to_string(),
        label: "System Default".to_string(),
    });
    
    Ok(final_devices)
}

pub fn lookup_device(target_id: Option<String>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    
    // Normalize "default" ID to None to trigger system fallback logic
    let target_id = target_id.filter(|id| id != "default");

    if let Some(id) = target_id {
        if id.starts_with("pulse:") {
            let pulse_source_name = &id[6..];
            #[cfg(target_os = "linux")]
            {
                std::env::set_var("PULSE_SOURCE", pulse_source_name);
                println!("🔧 Selected PulseAudio source via env: {}", pulse_source_name);
            }
            
            let devices = host.input_devices().map_err(|e| e.to_string())?;
            devices.into_iter().find(|d| {
                d.id().map(|id| id.1 == "pulse").unwrap_or(false)
            }).ok_or_else(|| "Could not find 'pulse' ALSA device".to_string())
        } else {
            let devices = host.input_devices().map_err(|e| e.to_string())?;
            for dev in devices {
                if let Ok(dev_id) = dev.id() {
                    if dev_id.1 == id {
                        return Ok(dev);
                    }
                }
            }
            Err(format!("Device '{}' not found", id))
        }
    } else {
        // If no specific device (or "default" selected), ensure PULSE_SOURCE is cleared on Linux
        // to follow system default (e.g. set via KDE)
        #[cfg(target_os = "linux")]
        {
            std::env::remove_var("PULSE_SOURCE");
            println!("🔧 Cleared PULSE_SOURCE to follow system default");
        }

        #[cfg(target_os = "linux")]
        {
            let devices = host.input_devices().map_err(|e| e.to_string())?;
            for dev in devices {
                if let Ok(id) = dev.id() {
                    let name = id.1;
                    if name == "pulse" || name.starts_with("default") {
                        return Ok(dev);
                    }
                }
            }
        }
        host.default_input_device().ok_or_else(|| "No input device available".to_string())
    }
}

pub async fn record_audio_while_flag(
    is_recording: &Arc<Mutex<bool>>,
    cached_device: Arc<Mutex<Option<cpal::Device>>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Try to get device from cache, or lookup if empty
    let device = {
        let guard = cached_device.lock().unwrap();
        if let Some(dev) = guard.as_ref() {
            dev.clone()
        } else {
            drop(guard);
            let dev = lookup_device(None)?;
            let mut guard = cached_device.lock().unwrap();
            *guard = Some(dev.clone());
            dev
        }
    };

    let final_device_name = match device.description() {
        Ok(desc) => desc.name().to_string(),
        Err(_) => device.id().map(|id| id.1).unwrap_or_else(|_| "Unknown".into()),
    };
    println!("⏺️  STARTING RECORDING using device: {}", final_device_name);

    let default_config = device.default_input_config()?;
    let config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    
    let temp_path = std::env::temp_dir().join("voquill_recording.wav");
    let spec = WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = WavWriter::create(&temp_path, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_2 = writer.clone();

    let err_fn = |err| {
        log::error!("Audio stream error: {}", err);
    };

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

    while {
        let recording = is_recording.lock().unwrap();
        *recording
    } {
        std::thread::sleep(Duration::from_millis(10));
    }

    drop(stream);

    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize()?;
        }
    }

    let audio_data = std::fs::read(&temp_path)?;
    let _ = std::fs::remove_file(&temp_path);
    let optimized_audio = convert_audio_for_whisper(&audio_data, config.sample_rate, config.channels)?;
    
    Ok(optimized_audio)
}


fn write_input_data_f32(input: &[f32], writer: &Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>) {
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
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
    if original_sample_rate == 16000 && original_channels == 1 {
        return Ok(audio_data.to_vec());
    }
    
    let mut cursor = std::io::Cursor::new(audio_data);
    let mut reader = hound::WavReader::new(&mut cursor)?;
    
    let spec = reader.spec();
    let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let samples = samples?;
    
    let mono_samples = if spec.channels == 2 {
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
    
    let resampled_samples = if spec.sample_rate != 16000 {
        resample_audio(&mono_samples, spec.sample_rate, 16000)
    } else {
        mono_samples
    };
    
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
