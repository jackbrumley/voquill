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
    pub recording_tx: Arc<Mutex<Option<mpsc::SyncSender<f32>>>>,
    pub sample_rate: u32,
}

impl PersistentAudioEngine {
    pub fn new(device: &cpal::Device, sensitivity: f32) -> Result<Self, String> {
        let stream_config = StreamConfig {
            channels: 1,
            sample_rate: 16000,
            buffer_size: cpal::BufferSize::Default,
        };

        let pre_roll_size = (16000.0 * 0.2) as usize;
        let pre_roll_rb = HeapRb::<f32>::new(pre_roll_size);
        let (mut pre_roll_prod, pre_roll_cons) = pre_roll_rb.split();

        let recording_tx = Arc::new(Mutex::new(None::<mpsc::SyncSender<f32>>));
        let recording_tx_clone = recording_tx.clone();

        let mut dc_offset_state = 0.0f32;
        let alpha = 0.995f32; 

        let err_fn = |err| println!("❌ Audio stream error: {}", err);
        
        let callback = {
            let recording_tx = recording_tx_clone.clone();
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for &sample_raw in data {
                    let filtered = sample_raw - dc_offset_state;
                    dc_offset_state = filtered + alpha * dc_offset_state;
                    let sample = filtered * sensitivity;

                    let _ = pre_roll_prod.try_push(sample);

                    if let Ok(guard) = recording_tx.try_lock() {
                        if let Some(tx) = guard.as_ref() {
                            let _ = tx.try_send(sample);
                        }
                    }
                }
            }
        };

        let stream = match device.build_input_stream(&stream_config, callback, err_fn, None) {
            Ok(s) => s,
            Err(e) => {
                println!("⚠️  16kHz Mono request failed ({}), falling back to default...", e);
                let config = device.default_input_config().map_err(|e| e.to_string())?;
                let channels = config.channels();
                let sensitivity = sensitivity;
                let recording_tx = recording_tx_clone.clone();
                let mut pre_roll_prod = {
                    let (prod, _) = HeapRb::<f32>::new(pre_roll_size).split();
                    prod
                };
                
                let mut dc_state_fallback = 0.0f32;
                let mut fallback_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for frame in data.chunks(channels as usize) {
                        let sum: f32 = frame.iter().sum();
                        let sample_raw = sum / channels as f32;
                        let filtered = sample_raw - dc_state_fallback;
                        dc_state_fallback = filtered + 0.995 * dc_state_fallback;
                        let sample = filtered * sensitivity;

                        let _ = pre_roll_prod.try_push(sample);
                        if let Ok(guard) = recording_tx.try_lock() {
                            if let Some(tx) = guard.as_ref() {
                                let _ = tx.try_send(sample);
                            }
                        }
                    }
                };

                match config.sample_format() {
                    SampleFormat::F32 => device.build_input_stream(&config.into(), fallback_callback, err_fn, None),
                    SampleFormat::I16 => device.build_input_stream(&config.into(), move |data: &[i16], info| {
                        let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        fallback_callback(&f32_data, info);
                    }, err_fn, None),
                    _ => return Err("Unsupported format".into()),
                }.map_err(|e| e.to_string())?
            }
        };

        stream.play().map_err(|e| e.to_string())?;

        Ok(Self {
            stream,
            pre_roll_consumer: Arc::new(Mutex::new(pre_roll_cons)),
            recording_tx: recording_tx_clone,
            sample_rate: 16000,
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
        if name.to_lowercase().contains(".monitor") || description.to_lowercase().contains("monitor") { continue; }
        devices.push(AudioDevice { id: format!("pulse:{}", name), label: description });
    }
    Ok(devices)
}

pub fn get_input_devices() -> Result<Vec<AudioDevice>, String> {
    let mut final_devices = Vec::new();
    #[cfg(target_os = "linux")]
    { if let Ok(devices) = get_linux_pulse_devices() { final_devices = devices; } }

    if final_devices.is_empty() {
        let mut seen = std::collections::HashSet::new();
        for host_id in cpal::available_hosts() {
            if let Ok(host) = cpal::host_from_id(host_id) {
                if let Ok(devices) = host.input_devices() {
                    for dev in devices {
                        let id = match dev.id() { Ok(id) => id.1, Err(_) => continue };
                        if !id.starts_with("default:") && id != "pulse" && id != "default" { continue; }
                        let label = match dev.description() { Ok(desc) => desc.name().to_string(), Err(_) => id.clone() };
                        if seen.insert(label.clone()) { final_devices.push(AudioDevice { id, label }); }
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
    let target = target_id.filter(|id| id != "default");

    if let Some(name) = target {
        if name.starts_with("pulse:") {
            #[cfg(target_os = "linux")]
            { std::env::set_var("PULSE_SOURCE", &name[6..]); }
            host.input_devices().map_err(|e| e.to_string())?.into_iter().find(|d| d.id().map(|id| id.1 == "pulse").unwrap_or(false)).ok_or_else(|| "Pulse ALSA device not found".to_string())
        } else {
            host.input_devices().map_err(|e| e.to_string())?.into_iter().find(|d| d.id().map(|id| id.1 == name).unwrap_or(false)).ok_or_else(|| format!("Device '{}' not found", name))
        }
    } else {
        #[cfg(target_os = "linux")]
        {
            if let Ok(devices) = host.input_devices() {
                for dev in devices {
                    if let Ok(id) = dev.id() {
                        if id.1 == "pulse" || id.1.starts_with("default") { return Ok(dev); }
                    }
                }
            }
        }
        host.default_input_device().ok_or_else(|| "No input device available".to_string())
    }
}

pub async fn record_audio_while_flag(
    is_recording: &Arc<Mutex<bool>>,
    engine: Arc<Mutex<Option<PersistentAudioEngine>>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    println!("🎤 record_audio_while_flag: Initializing sync channel");
    let (tx, rx) = mpsc::sync_channel::<f32>(65536);
    let mut samples = Vec::new();
    let sample_rate;
    
    {
        let mut guard = engine.lock().unwrap();
        let eng = guard.as_mut().ok_or("Audio engine not initialized")?;
        sample_rate = eng.sample_rate;
        println!("🎤 record_audio_while_flag: Using sample rate {}", sample_rate);
        if let Ok(mut cons) = eng.pre_roll_consumer.lock() {
            let pre_roll_count = samples.len();
            while let Some(s) = cons.try_pop() { samples.push(s); }
            println!("🎤 record_audio_while_flag: Collected {} pre-roll samples", samples.len() - pre_roll_count);
        }
        *eng.recording_tx.lock().unwrap() = Some(tx);
    }

    let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        println!("🎤 record_audio_while_flag: Processing thread started");
        let mut all = samples;
        let mut received = 0;
        while let Ok(s) = rx.recv() { 
            all.push(s); 
            received += 1;
        }
        println!("🎤 record_audio_while_flag: Finished receiving. Total samples: {}, Received during recording: {}", all.len(), received);
        let mut out = Vec::new();
        if let Ok(mut w) = WavWriter::new(std::io::Cursor::new(&mut out), WavSpec { channels: 1, sample_rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int }) {
            for s in all { let _ = w.write_sample(process_sample(s)); }
            let _ = w.finalize();
        }
        println!("🎤 record_audio_while_flag: WAV encoding complete ({} bytes)", out.len());
        let _ = data_tx.send(out);
    });

    println!("🎤 record_audio_while_flag: Waiting for is_recording to become false...");
    while *is_recording.lock().unwrap() { tokio::time::sleep(Duration::from_millis(10)).await; }
    println!("🎤 record_audio_while_flag: is_recording is false, stopping recording_tx");
    {
        if let Some(eng) = engine.lock().unwrap().as_ref() { *eng.recording_tx.lock().unwrap() = None; }
    }

    println!("🎤 record_audio_while_flag: Waiting for processed data...");
    let final_wav = data_rx.recv()?;
    println!("🎤 record_audio_while_flag: Data received, converting for Whisper");
    Ok(convert_audio_for_whisper(&final_wav, sample_rate, 1)?)
}

pub async fn record_mic_test<F>(
    is_mic_test: &Arc<Mutex<bool>>,
    engine: Arc<Mutex<Option<PersistentAudioEngine>>>,
    on_volume: F,
) -> Result<Vec<i16>, Box<dyn std::error::Error + Send + Sync>> 
where F: Fn(f32) + Send + 'static
{
    let (tx, rx) = mpsc::sync_channel::<f32>(65536);
    let sample_rate;
    {
        let mut guard = engine.lock().unwrap();
        let eng = guard.as_mut().ok_or("Audio engine not initialized")?;
        sample_rate = eng.sample_rate;
        *eng.recording_tx.lock().unwrap() = Some(tx);
    }

    let (data_tx, data_rx) = mpsc::channel::<Vec<i16>>();
    std::thread::spawn(move || {
        let mut samples = Vec::new();
        let mut peak = 0.0f32;
        let mut count = 0;
        let throttle_window = 800; // ~50ms at 16kHz

        while let Ok(s) = rx.recv() {
            let abs_s = s.abs();
            if abs_s > peak { peak = abs_s; }
            count += 1;

            if count >= throttle_window {
                on_volume(peak);
                peak = 0.0;
                count = 0;
            }
            
            samples.push(process_sample(s));
        }
        let _ = data_tx.send(samples);
    });

    while *is_mic_test.lock().unwrap() { tokio::time::sleep(Duration::from_millis(10)).await; }
    {
        if let Some(eng) = engine.lock().unwrap().as_ref() { *eng.recording_tx.lock().unwrap() = None; }
    }

    let mut final_samples = data_rx.recv()?;
    if sample_rate != 16000 { final_samples = resample_audio(&final_samples, sample_rate, 16000); }
    normalize_audio(&mut final_samples);
    Ok(final_samples)
}

pub fn play_audio<F>(samples: Vec<i16>, sample_rate: u32, on_done: F) -> Result<cpal::Stream, Box<dyn std::error::Error + Send + Sync>> 
where F: FnOnce() + Send + 'static
{
    let host = cpal::default_host();
    let device = {
        let mut selected = None;
        if let Ok(devices) = host.output_devices() {
            for dev in devices {
                if let Ok(id) = dev.id() {
                    if id.1 == "pulse" || id.1.starts_with("default") { selected = Some(dev); break; }
                }
            }
        }
        selected.or_else(|| host.default_output_device())
    }.ok_or("No output device available")?;

    let config = device.default_output_config()?;
    let stream_config: StreamConfig = config.clone().into();
    let resampled = Arc::new(resample_audio(&samples, sample_rate, stream_config.sample_rate));
    let chans = stream_config.channels as usize;

    let resampled_clone1 = resampled.clone();
    let mut idx1 = 0;
    let mut done1 = Some(on_done);
    let callback = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        for frame in data.chunks_mut(chans) {
            if idx1 < resampled_clone1.len() {
                let s = resampled_clone1[idx1] as f32 / i16::MAX as f32;
                for out in frame.iter_mut() { *out = s; }
                idx1 += 1;
            } else {
                for out in frame.iter_mut() { *out = 0.0; }
                if let Some(cb) = done1.take() { cb(); }
            }
        }
    };

    let err_fn = |err| println!("🔊 Playback error: {}", err);
    let stream = match config.sample_format() {
        SampleFormat::F32 => device.build_output_stream(&stream_config, callback, err_fn, None)?,
        SampleFormat::I16 => {
            let resampled_clone2 = resampled.clone();
            let mut idx2 = 0;
            device.build_output_stream(&stream_config, move |data: &mut [i16], _| {
                for frame in data.chunks_mut(chans) {
                    if idx2 < resampled_clone2.len() {
                        let s = resampled_clone2[idx2];
                        for out in frame.iter_mut() { *out = s; }
                        idx2 += 1;
                    } else {
                        for out in frame.iter_mut() { *out = 0; }
                    }
                }
            }, err_fn, None)?
        },
        _ => return Err("Unsupported format".into()),
    };
    stream.play()?;
    Ok(stream)
}

fn convert_audio_for_whisper(data: &[u8], rate: u32, _chans: u16) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if rate == 16000 { return Ok(data.to_vec()); }
    let mut reader = hound::WavReader::new(std::io::Cursor::new(data))?;
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap_or(0)).collect();
    let mut mono = if reader.spec().channels == 2 { samples.chunks(2).map(|c| if c.len()==2 {((c[0] as i32 + c[1] as i32)/2) as i16} else {c[0]}).collect() } else { samples };
    if reader.spec().sample_rate != 16000 { mono = resample_audio(&mono, reader.spec().sample_rate, 16000); }
    let mut out = Vec::new();
    {
        let mut w = WavWriter::new(std::io::Cursor::new(&mut out), WavSpec { channels: 1, sample_rate: 16000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int })?;
        for s in mono { w.write_sample(s)?; }
        w.finalize()?;
    }
    Ok(out)
}

pub fn normalize_audio(samples: &mut [i16]) {
    let max = samples.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0);
    if max > 0 && max < (i16::MAX as i32 / 2) {
        let gain = (i16::MAX as f32 * 0.9) / max as f32;
        for s in samples.iter_mut() { *s = ((*s as f32 * gain) as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16; }
    }
}

pub fn resample_audio(samples: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from == to { return samples.to_vec(); }
    let ratio = from as f64 / to as f64;
    let len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
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

fn process_sample(s: f32) -> i16 {
    let clipped = soft_clip(s);
    (clipped * 0.95 * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn soft_clip(x: f32) -> f32 {
    if x.abs() <= 0.7 { x }
    else if x > 0.7 { 0.7 + 0.3 * ((x - 0.7) / 0.3).tanh() }
    else { -0.7 - 0.3 * ((-x - 0.7) / 0.3).tanh() }
}
