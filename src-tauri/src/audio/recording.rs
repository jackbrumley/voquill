use std::sync::{mpsc, Arc, Mutex};

use ringbuf::traits::Consumer;

use crate::app::state::SessionState;

use super::conversion::finalize_captured_audio_for_whisper;
use super::engine::PersistentAudioEngine;

pub async fn record_audio_while_flag(
    session_state: &Arc<Mutex<SessionState>>,
    engine: Arc<Mutex<Option<PersistentAudioEngine>>>,
    post_roll_ms: u64,
    max_recording_duration: std::time::Duration,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    crate::log_info!("record_audio_while_flag: enter");
    let (tx, rx) = mpsc::sync_channel::<f32>(65536);
    let mut samples = Vec::new();
    let sample_rate;
    {
        let mut guard = engine.lock().unwrap();
        let eng = guard.as_mut().ok_or("Audio engine not initialized")?;
        sample_rate = eng.sample_rate;
        // Drain pre-roll samples accumulated before recording started
        if let Ok(mut cons) = eng.pre_roll_consumer.lock() {
            while let Some(s) = cons.try_pop() {
                samples.push(s);
            }
        }
        *eng.recording_tx.lock().unwrap() = Some(tx);
    }

    let (data_tx, data_rx) = mpsc::channel::<Vec<f32>>();
    std::thread::spawn(move || {
        let mut all = samples;
        while let Ok(s) = rx.recv() {
            all.push(s);
        }
        let _ = data_tx.send(all);
    });

    let capture_started = tokio::time::Instant::now();
    loop {
        let still_recording = matches!(*session_state.lock().unwrap(), SessionState::Recording);
        if !still_recording {
            break;
        }
        if capture_started.elapsed() >= max_recording_duration {
            crate::log_warn!(
                "record_audio_while_flag: max recording duration of {:?} reached; auto-stopping capture",
                max_recording_duration
            );
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    crate::log_info!("record_audio_while_flag: capture loop ended, finalizing capture");

    if post_roll_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(post_roll_ms)).await;
        crate::log_info!("record_audio_while_flag: post-roll of {post_roll_ms}ms complete");
    }

    // Close the recording channel — drops the sender in the callback,
    // which makes rx.recv() return Err, terminating the drain thread.
    if let Some(eng) = engine.lock().unwrap().as_ref() {
        *eng.recording_tx.lock().unwrap() = None;
    }

    let raw_samples = data_rx.recv()?;
    crate::log_info!(
        "record_audio_while_flag: captured {} raw float samples at {}Hz",
        raw_samples.len(),
        sample_rate
    );
    finalize_captured_audio_for_whisper(&raw_samples, sample_rate)
}

pub async fn record_mic_test<F>(
    is_mic_test: &Arc<Mutex<bool>>,
    engine: Arc<Mutex<Option<PersistentAudioEngine>>>,
    on_volume: F,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>
where
    F: Fn(f32) + Send + 'static,
{
    let (tx, rx) = mpsc::sync_channel::<f32>(65536);
    let sample_rate;
    {
        let mut guard = engine.lock().unwrap();
        let eng = guard.as_mut().ok_or("Audio engine not initialized")?;
        sample_rate = eng.sample_rate;
        *eng.recording_tx.lock().unwrap() = Some(tx);
    }

    let (data_tx, data_rx) = mpsc::channel::<Vec<f32>>();
    std::thread::spawn(move || {
        let mut samples = Vec::new();
        let mut peak = 0.0f32;
        let mut count = 0;
        let throttle_window = 800;
        while let Ok(s) = rx.recv() {
            let abs_s = s.abs();
            if abs_s > peak {
                peak = abs_s;
            }
            count += 1;
            if count >= throttle_window {
                on_volume(peak);
                peak = 0.0;
                count = 0;
            }
            samples.push(s);
        }
        let _ = data_tx.send(samples);
    });

    while *is_mic_test.lock().unwrap() {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    if let Some(eng) = engine.lock().unwrap().as_ref() {
        *eng.recording_tx.lock().unwrap() = None;
    }
    let final_samples = data_rx.recv()?;

    crate::log_info!(
        "Mic test: Finished with {} samples at {}Hz",
        final_samples.len(),
        sample_rate
    );

    Ok(final_samples)
}
