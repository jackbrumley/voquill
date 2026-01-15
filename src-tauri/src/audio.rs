use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use ringbuf::traits::*;
use ringbuf::{HeapRb, CachingCons};

#[cfg(target_os = "linux")]
use pulsectl::controllers::DeviceControl;

#[derive(serde::Serialize, Clone, Debug)]
pub struct AudioDevice {
    pub id: String,
    pub label: String,
}

pub struct PersistentAudioEngine {
    pub stream: cpal::Stream,
    pub pre_roll_consumer: Arc<Mutex<CachingCons<Arc<HeapRb<f32>>>>>,
    pub recording_tx: Arc<Mutex<Option<mpsc::Sender<f32>>>>,
    pub sample_rate: u32,
}

impl PersistentAudioEngine {
    pub fn new(device: &cpal::Device, sensitivity: f32) -> Result<Self, String> {
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        let sample_rate = config.sample_rate();
        let channels = config.channels();
        
        let stream_config = StreamConfig {
            channels,
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        // 200ms pre-roll buffer
        let pre_roll_size = (sample_rate as f32 * 0.2) as usize;
        let pre_roll_rb = HeapRb::<f32>::new(pre_roll_size);
        let (mut pre_roll_prod, pre_roll_cons) = pre_roll_rb.split();

        let recording_tx = Arc::new(Mutex::new(None::<mpsc::Sender<f32>>));
        let recording_tx_clone = recording_tx.clone();

        // DC Offset Filter state
        let mut dc_offset_state = 0.0f32;
        let alpha = 0.995f32; // Time constant for high-pass filter

        let err_fn = |err| println!("❌ Audio stream error: {}", err);
        
        let mut callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            for frame in data.chunks(channels as usize) {
                // Average channels for mono
                let sum: f32 = frame.iter().sum();
                let sample_raw = sum / channels as f32;

                // 1. High-Pass Filter (DC Offset Removal)
                let filtered = sample_raw - dc_offset_state;
                dc_offset_state = filtered + alpha * dc_offset_state;
                let mut sample = filtered;

                // 2. Sensitivity / Gain
                sample *= sensitivity;

                // Push to pre-roll (non-blocking)
                let _ = pre_roll_prod.try_push(sample);

                // Push to active recording if any
                if let Ok(guard) = recording_tx_clone.try_lock() {
                    if let Some(tx) = guard.as_ref() {
                        let _ = tx.send(sample);
                    }
                }
            }
        };

        let stream = match config.sample_format() {
            SampleFormat::F32 => {
                device.build_input_stream(&stream_config, callback, err_fn, None)
            }
            SampleFormat::I16 => {
                device.build_input_stream(&stream_config, move |data: &[i16], info| {
                    let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    callback(&f32_data, info);
                }, err_fn, None)
            }
            _ => return Err("Unsupported sample format".into()),
        }.map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(Self {
            stream,
            pre_roll_consumer: Arc::new(Mutex::new(pre_roll_cons)),
            recording_tx,
            sample_rate,
        })
    }
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
            // std::env::remove_var("PULSE_SOURCE"); // Don't remove if we might be using it
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

fn process_sample(sample: f32) -> i16 {
    let clipped = soft_clip(sample);
    let with_headroom = clipped * 0.95;
    (with_headroom * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

pub async fn record_audio_while_flag(
    is_recording: &Arc<Mutex<bool>>,
    engine: Arc<Mutex<Option<PersistentAudioEngine>>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let (tx, rx) = mpsc::channel::<f32>();
    let sample_rate;
    
    // Setup recording and capture pre-roll
    let mut initial_samples = Vec::new();
    {
        let mut guard = engine.lock().unwrap();
        let eng = guard.as_mut().ok_or("Audio engine not initialized")?;
        sample_rate = eng.sample_rate;
        
        // 1. Grab pre-roll data
        if let Ok(mut cons) = eng.pre_roll_consumer.lock() {
            while let Some(s) = cons.try_pop() {
                initial_samples.push(s);
            }
        }
        
        // 2. Set live channel
        let mut rec_tx = eng.recording_tx.lock().unwrap();
        *rec_tx = Some(tx);
    }

    let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>();

    // Collector thread
    std::thread::spawn(move || {
        let mut samples = initial_samples;
        while let Ok(sample) = rx.recv() {
            samples.push(sample);
        }
        
        // Convert to WAV format for Whisper
        let mut out = Vec::new();
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        if let Ok(mut writer) = WavWriter::new(std::io::Cursor::new(&mut out), spec) {
            for s in samples {
                let _ = writer.write_sample(process_sample(s));
            }
            let _ = writer.finalize();
        }
        
        let _ = data_tx.send(out);
    });

    // Wait for flag to flip
    while *is_recording.lock().unwrap() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Stop recording
    {
        let guard = engine.lock().unwrap();
        if let Some(eng) = guard.as_ref() {
            let mut rec_tx = eng.recording_tx.lock().unwrap();
            *rec_tx = None;
        }
    }

    let final_wav = data_rx.recv()?;
    
    // Finally, ensure it is 16kHz for whisper
    Ok(convert_audio_for_whisper(&final_wav, sample_rate, 1)?)
}

pub async fn record_mic_test<F>(
    is_recording: &Arc<Mutex<bool>>,
    engine: Arc<Mutex<Option<PersistentAudioEngine>>>,
    on_volume: F,
) -> Result<Vec<i16>, Box<dyn std::error::Error + Send + Sync>> 
where F: Fn(f32) + Send + 'static
{
    let (tx, rx) = mpsc::channel::<f32>();
    let sample_rate;
    
    {
        let mut guard = engine.lock().unwrap();
        let eng = guard.as_mut().ok_or("Audio engine not initialized")?;
        sample_rate = eng.sample_rate;
        let mut rec_tx = eng.recording_tx.lock().unwrap();
        *rec_tx = Some(tx);
    }

    let (data_tx, data_rx) = mpsc::channel::<Vec<i16>>();

    std::thread::spawn(move || {
        let mut samples = Vec::new();
        while let Ok(sample) = rx.recv() {
            let abs_sample = sample.abs();
            on_volume(abs_sample);
            samples.push(process_sample(sample));
        }
        let _ = data_tx.send(samples);
    });

    while *is_recording.lock().unwrap() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    {
        let guard = engine.lock().unwrap();
        if let Some(eng) = guard.as_ref() {
            let mut rec_tx = eng.recording_tx.lock().unwrap();
            *rec_tx = None;
        }
    }

    let mut final_samples = data_rx.recv()?;
    if sample_rate != 16000 {
        final_samples = resample_audio(&final_samples, sample_rate, 16000);
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
