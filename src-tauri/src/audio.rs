use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex, mpsc};
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
    let mut handler = pulsectl::controllers::SourceController::create()
        .map_err(|e| format!("Failed to connect to PulseAudio: {}", e))?;
    let sources = handler.list_devices()
        .map_err(|e| format!("Failed to list PulseAudio sources: {}", e))?;

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
    let mut final_devices = Vec::new();
    #[cfg(target_os = "linux")]
    {
        if let Ok(devices) = get_linux_pulse_devices() {
            final_devices = devices;
        }
    }

    if final_devices.is_empty() {
        let mut seen_labels = std::collections::HashSet::new();
        for host_id in cpal::available_hosts() {
            if let Ok(host) = cpal::host_from_id(host_id) {
                if let Ok(devices) = host.input_devices() {
                    for dev in devices {
                        let device_id = match dev.id() { Ok(id) => id.1, Err(_) => continue };
                        if !device_id.starts_with("default:") && device_id != "pulse" && device_id != "default" { continue; }
                        let label = match dev.description() { Ok(desc) => desc.name().to_string(), Err(_) => device_id.clone() };
                        let clean_label = label.split(", USB Audio").next().unwrap_or(&label).split(", ALC").next().unwrap_or(&label).trim().to_string();
                        if !seen_labels.contains(&clean_label) {
                            final_devices.push(AudioDevice { id: device_id.clone(), label: clean_label.clone() });
                            seen_labels.insert(clean_label);
                        }
                    }
                }
            }
        }
    }

    final_devices.sort_by(|a, b| a.label.cmp(&b.label));
    final_devices.insert(0, AudioDevice { id: "default".to_string(), label: "System Default".to_string() });
    Ok(final_devices)
}

pub fn lookup_device(target_id: Option<String>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    let target_id_string = target_id.filter(|id| id != "default");

    if let Some(target_name) = target_id_string {
        if target_name.starts_with("pulse:") {
            let pulse_source_name = &target_name[6..];
            #[cfg(target_os = "linux")]
            {
                std::env::set_var("PULSE_SOURCE", pulse_source_name);
            }
            host.input_devices().map_err(|e| e.to_string())?
                .into_iter().find(|d| d.id().map(|id| id.1 == "pulse").unwrap_or(false))
                .ok_or_else(|| "Could not find 'pulse' ALSA device".to_string())
        } else {
            host.input_devices().map_err(|e| e.to_string())?
                .into_iter().find(|d| d.id().map(|id| id.1 == target_name).unwrap_or(false))
                .ok_or_else(|| format!("Device '{}' not found", target_name))
        }
    } else {
        #[cfg(target_os = "linux")]
        {
            std::env::remove_var("PULSE_SOURCE");
            if let Ok(devices) = host.input_devices() {
                for dev in devices {
                    if let Ok(id) = dev.id() {
                        let name = id.1;
                        if name == "pulse" || name.starts_with("default") { return Ok(dev); }
                    }
                }
            }
        }
        host.default_input_device().ok_or_else(|| "No input device available".to_string())
    }
}

fn soft_clip(x: f32) -> f32 {
    if x.abs() <= 0.7 {
        x
    } else if x > 0.7 {
        0.7 + 0.3 * ((x - 0.7) / 0.3).tanh()
    } else {
        -0.7 - 0.3 * ((-x - 0.7) / 0.3).tanh()
    }
}

fn process_sample(sample: f32, sensitivity: f32) -> i16 {
    let amplified = sample * sensitivity;
    let clipped = soft_clip(amplified);
    let with_headroom = clipped * 0.95;
    (with_headroom * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

pub async fn record_audio_while_flag(
    is_recording: &Arc<Mutex<bool>>,
    cached_device: Arc<Mutex<Option<cpal::Device>>>,
    sensitivity: f32,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let device = {
        let mut guard = cached_device.lock().unwrap();
        if let Some(dev) = guard.as_ref() { dev.clone() }
        else {
            let dev = lookup_device(None)?;
            *guard = Some(dev.clone());
            dev
        }
    };

    let default_config = device.default_input_config()?;
    let stream_config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    
    let temp_path = std::env::temp_dir().join("voquill_recording.wav");
    let spec = WavSpec {
        channels: stream_config.channels,
        sample_rate: stream_config.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(100);
    let writer = WavWriter::create(&temp_path, spec)?;
    let mut writer = Some(writer);

    let writer_thread = std::thread::spawn(move || {
        let mut w = writer.take().unwrap();
        while let Ok(data) = rx.recv() {
            for sample in data {
                w.write_sample(process_sample(sample, sensitivity)).unwrap();
            }
        }
        let _ = w.finalize();
    });

    let err_fn = |err| println!("❌ Audio stream error: {}", err);
    let tx_clone = tx.clone();
    let stream = match default_config.sample_format() {
        SampleFormat::F32 => device.build_input_stream(&stream_config, move |data: &[f32], _| { let _ = tx_clone.try_send(data.to_vec()); }, err_fn, None)?,
        SampleFormat::I16 => device.build_input_stream(&stream_config, move |data: &[i16], _| {
            let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
            let _ = tx_clone.try_send(f32_data);
        }, err_fn, None)?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;
    while *is_recording.lock().unwrap() { std::thread::sleep(Duration::from_millis(10)); }
    drop(stream);
    drop(tx);
    let _ = writer_thread.join();

    let audio_data = std::fs::read(&temp_path)?;
    let _ = std::fs::remove_file(&temp_path);
    Ok(convert_audio_for_whisper(&audio_data, stream_config.sample_rate, stream_config.channels)?)
}

pub async fn record_mic_test<F>(
    is_recording: &Arc<Mutex<bool>>,
    cached_device: Arc<Mutex<Option<cpal::Device>>>,
    sensitivity: f32,
    on_volume: F,
) -> Result<Vec<i16>, Box<dyn std::error::Error + Send + Sync>> 
where F: Fn(f32) + Send + 'static
{
    let device = {
        let mut guard = cached_device.lock().unwrap();
        if let Some(dev) = guard.as_ref() { dev.clone() }
        else {
            let dev = lookup_device(None)?;
            *guard = Some(dev.clone());
            dev
        }
    };

    let default_config = device.default_input_config()?;
    let stream_config = StreamConfig {
        channels: default_config.channels(),
        sample_rate: default_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    
    let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(100);
    let collector_thread = std::thread::spawn(move || {
        let mut all_samples = Vec::new();
        while let Ok(data) = rx.recv() {
            let mut peak: f32 = 0.0;
            for sample in data {
                let amplified = sample * sensitivity;
                let abs_sample = amplified.abs();
                if abs_sample > peak { peak = abs_sample; }
                all_samples.push(process_sample(sample, sensitivity));
            }
            on_volume(peak);
        }
        all_samples
    });

    let err_fn = |err| println!("🎤 record_mic_test: Audio stream error: {}", err);
    let tx_clone = tx.clone();
    let stream = match default_config.sample_format() {
        SampleFormat::F32 => device.build_input_stream(&stream_config, move |data: &[f32], _| { let _ = tx_clone.try_send(data.to_vec()); }, err_fn, None)?,
        SampleFormat::I16 => device.build_input_stream(&stream_config, move |data: &[i16], _| {
            let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
            let _ = tx_clone.try_send(f32_data);
        }, err_fn, None)?,
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;
    while *is_recording.lock().unwrap() { std::thread::sleep(Duration::from_millis(10)); }
    drop(stream);
    drop(tx);

    let mut final_samples = collector_thread.join().unwrap();
    if stream_config.channels == 2 {
        final_samples = final_samples.chunks(2).map(|chunk| if chunk.len() == 2 { ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16 } else { chunk[0] }).collect();
    }
    if stream_config.sample_rate != 16000 {
        final_samples = resample_audio(&final_samples, stream_config.sample_rate, 16000);
    }
    normalize_audio(&mut final_samples);
    Ok(final_samples)
}

pub fn play_audio<F>(samples: Vec<i16>, sample_rate: u32, on_done: F) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>> 
where F: FnOnce() + Send + 'static
{
    let host = cpal::default_host();
    #[cfg(target_os = "linux")]
    let device = {
        let mut selected_dev = None;
        if let Ok(devices) = host.output_devices() {
            for dev in devices {
                if let Ok(id) = dev.id() {
                    let name = id.1;
                    if name == "pulse" || name.starts_with("default") { selected_dev = Some(dev); break; }
                }
            }
        }
        selected_dev.or_else(|| host.default_output_device())
    }.ok_or("No output device available")?;
    #[cfg(not(target_os = "linux"))]
    let device = host.default_output_device().ok_or("No output device available")?;

    let config = device.default_output_config()?;
    let sample_format = config.sample_format();
    let stream_config: StreamConfig = config.into();

    let resampled_samples = Arc::new(resample_audio(&samples, sample_rate, stream_config.sample_rate));
    let mut sample_index = 0;
    let mut on_done = Some(on_done);
    let channels = stream_config.channels as usize;


    let err_fn = |err| println!("🔊 play_audio: Playback error: {}", err);
    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(&stream_config, move |data: &mut [f32], _| {
            for frame in data.chunks_mut(channels) {
                if sample_index < resampled_samples.len() {
                    let s = resampled_samples[sample_index] as f32 / i16::MAX as f32;
                    for out in frame.iter_mut() { *out = s; }
                    sample_index += 1;
                } else {
                    for out in frame.iter_mut() { *out = 0.0; }
                    if let Some(cb) = on_done.take() { cb(); }
                }
            }
        }, err_fn, None)?,
        SampleFormat::I16 => device.build_output_stream(&stream_config, move |data: &mut [i16], _| {
            for frame in data.chunks_mut(channels) {
                if sample_index < resampled_samples.len() {
                    let s = resampled_samples[sample_index];
                    for out in frame.iter_mut() { *out = s; }
                    sample_index += 1;
                } else {
                    for out in frame.iter_mut() { *out = 0; }
                    if let Some(cb) = on_done.take() { cb(); }
                }
            }
        }, err_fn, None)?,
        _ => return Err("Unsupported output sample format".into()),
    };
    stream.play()?;
    Ok(stream)
}

fn convert_audio_for_whisper(audio_data: &[u8], _original_sample_rate: u32, _original_channels: u16) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(audio_data))?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap_or(0)).collect();
    let mut mono = if spec.channels == 2 { samples.chunks(2).map(|c| if c.len()==2 {((c[0] as i32 + c[1] as i32)/2) as i16} else {c[0]}).collect() } else { samples };
    if spec.sample_rate != 16000 { mono = resample_audio(&mono, spec.sample_rate, 16000); }
    let mut out = Vec::new();
    {
        let mut w = WavWriter::new(std::io::Cursor::new(&mut out), WavSpec { channels: 1, sample_rate: 16000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int })?;
        for s in mono { w.write_sample(s)?; }
        w.finalize()?;
    }
    Ok(out)
}

pub fn normalize_audio(samples: &mut [i16]) {
    let max_abs = samples.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0);
    if max_abs > 0 && max_abs < (i16::MAX as i32 / 2) {
        let gain = (i16::MAX as f32 * 0.9) / max_abs as f32;
        for s in samples.iter_mut() { *s = ((*s as f32 * gain) as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16; }
    }
}

pub fn resample_audio(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate { return samples.to_vec(); }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = pos - idx as f64;
        if idx + 1 < samples.len() {
            let s1 = samples[idx] as f64;
            let s2 = samples[idx+1] as f64;
            out.push((s1 + (s2 - s1) * frac) as i16);
        } else if idx < samples.len() { out.push(samples[idx]); }
    }
    out
}
