use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

use crate::app::state::SessionState;
use crate::audio::conversion::finalize_captured_audio_for_whisper;
use crate::audio::decode::{decode_compressed_audio, DecodedAudio};
use crate::config::VoiceMacroCommand;
use crate::AppState;

const MACRO_TRIGGER_SOUND_BYTES: &[u8] = include_bytes!("../../sounds/macro_trigger.mp3");

static CACHED_SOUND: OnceLock<Option<DecodedAudio>> = OnceLock::new();

fn get_cached_sound() -> Option<&'static DecodedAudio> {
    CACHED_SOUND
        .get_or_init(
            || match decode_compressed_audio(MACRO_TRIGGER_SOUND_BYTES) {
                Ok(decoded) => {
                    crate::log_info!(
                        "Voice Macro sound decoded successfully ({} samples at {}Hz)",
                        decoded.samples.len(),
                        decoded.sample_rate
                    );
                    Some(decoded)
                }
                Err(e) => {
                    crate::log_warn!("Failed to decode embedded Voice Macro sound: {}", e);
                    None
                }
            },
        )
        .as_ref()
}

pub fn play_macro_trigger_sound(playback_device: Option<String>) {
    if let Some(decoded) = get_cached_sound() {
        let samples = decoded.samples.clone();
        let sample_rate = decoded.sample_rate;
        std::thread::spawn(move || {
            match crate::audio::playback::play_audio(samples, sample_rate, playback_device, || {}) {
                Ok(stream) => {
                    std::thread::sleep(Duration::from_millis(1500));
                    drop(stream);
                }
                Err(e) => {
                    crate::log_warn!("Voice Macro playback error: {}", e);
                }
            }
        });
    }
}

pub fn normalize_phrase(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_alphanumeric() || ch.is_whitespace() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(' ');
        }
    }
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MacroMatchResult {
    pub matched: bool,
    pub similarity: f32,
    pub transcript: String,
    pub matched_command: Option<VoiceMacroCommand>,
}

pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut prev_row: Vec<usize> = (0..=len_b).collect();
    let mut curr_row: Vec<usize> = vec![0; len_b + 1];

    for (i, &char_a) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, &char_b) in b_chars.iter().enumerate() {
            let cost = if char_a == char_b { 0 } else { 1 };
            curr_row[j + 1] = std::cmp::min(
                std::cmp::min(prev_row[j + 1] + 1, curr_row[j] + 1),
                prev_row[j] + cost,
            );
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[len_b]
}

pub fn string_similarity(a: &str, b: &str) -> f32 {
    let a_clean = normalize_phrase(a);
    let b_clean = normalize_phrase(b);

    if a_clean == b_clean {
        return 1.0;
    }

    let a_collapsed: String = a_clean.chars().filter(|c| !c.is_whitespace()).collect();
    let b_collapsed: String = b_clean.chars().filter(|c| !c.is_whitespace()).collect();

    if a_collapsed == b_collapsed {
        return 1.0;
    }

    let max_len = a_collapsed.len().max(b_collapsed.len());
    if max_len == 0 {
        return 1.0;
    }

    let dist = levenshtein_distance(&a_collapsed, &b_collapsed);
    (1.0 - (dist as f32 / max_len as f32)).max(0.0)
}

pub fn find_best_match(
    transcript: &str,
    trigger_word: &str,
    commands: &[VoiceMacroCommand],
) -> MacroMatchResult {
    let clean_transcript = normalize_phrase(transcript);
    let clean_trigger = normalize_phrase(trigger_word);

    let candidate_phrase = if !clean_trigger.is_empty() {
        if !clean_transcript.contains(&clean_trigger) {
            return MacroMatchResult {
                matched: false,
                similarity: 0.0,
                transcript: clean_transcript,
                matched_command: None,
            };
        }
        clean_transcript.replacen(&clean_trigger, "", 1)
    } else {
        clean_transcript.clone()
    };

    let clean_candidate = candidate_phrase.trim();
    if clean_candidate.is_empty() {
        return MacroMatchResult {
            matched: false,
            similarity: 0.0,
            transcript: clean_transcript,
            matched_command: None,
        };
    }

    let mut best_cmd: Option<VoiceMacroCommand> = None;
    let mut best_sim = 0.0f32;

    for command in commands {
        let clean_macro = normalize_phrase(&command.phrase);
        if clean_macro.is_empty() {
            continue;
        }

        // 1. Direct similarity (including space-insensitive collapse)
        let sim = string_similarity(clean_candidate, &clean_macro);
        if sim > best_sim {
            best_sim = sim;
            best_cmd = Some(command.clone());
        }

        // 2. Windowed similarity across words if candidate is longer than macro
        let candidate_words: Vec<&str> = clean_candidate.split_whitespace().collect();
        let macro_words: Vec<&str> = clean_macro.split_whitespace().collect();
        let m_len = macro_words.len();

        if candidate_words.len() >= m_len && m_len > 0 {
            let min_w = m_len.saturating_sub(1).max(1);
            let max_w = (m_len + 1).min(candidate_words.len());

            for w_len in min_w..=max_w {
                for window in candidate_words.windows(w_len) {
                    let window_str = window.join(" ");
                    let w_sim = string_similarity(&window_str, &clean_macro);
                    if w_sim > best_sim {
                        best_sim = w_sim;
                        best_cmd = Some(command.clone());
                    }
                }
            }
        }
    }

    let is_matched = best_sim >= 0.78;

    MacroMatchResult {
        matched: is_matched,
        similarity: best_sim,
        transcript: clean_transcript,
        matched_command: if is_matched { best_cmd } else { None },
    }
}

pub fn match_phrase(
    transcript: &str,
    trigger_word: &str,
    commands: &[VoiceMacroCommand],
) -> Option<VoiceMacroCommand> {
    let res = find_best_match(transcript, trigger_word, commands);
    if res.matched {
        res.matched_command
    } else {
        None
    }
}

pub fn resolve_voice_macro_prompt_hint(config: &crate::config::Config) -> Option<String> {
    let mut words: Vec<String> = config.dictionary.clone();
    for cmd in &config.voice_macros {
        let clean = cmd.phrase.trim();
        if !clean.is_empty() && !words.iter().any(|w| w.eq_ignore_ascii_case(clean)) {
            words.push(clean.to_string());
        }
    }
    if !config.voice_macro_trigger_word.trim().is_empty() {
        let trigger = config.voice_macro_trigger_word.trim();
        if !words.iter().any(|w| w.eq_ignore_ascii_case(trigger)) {
            words.push(trigger.to_string());
        }
    }

    if words.is_empty() {
        return None;
    }

    Some(format!("Vocabulary: {}.", words.join(", ")))
}

pub async fn execute_macro_command(
    app_handle: &AppHandle,
    command: &VoiceMacroCommand,
) -> Result<(), String> {
    let steps = command.resolve_steps();
    execute_macro_steps(app_handle, &command.phrase, &steps).await
}

pub async fn execute_macro_steps(
    app_handle: &AppHandle,
    phrase_label: &str,
    steps: &[crate::config::MacroStep],
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut held_keys: Vec<String> = Vec::new();

    crate::log_info!(
        "[Voice Macro] Executing command '{}' ({} steps)",
        phrase_label,
        steps.len()
    );

    let result = execute_macro_steps_inner(app_handle, steps, &mut held_keys).await;

    // Safety Guard: Release any keys still held down to prevent stuck keys!
    if !held_keys.is_empty() {
        crate::log_info!(
            "[Voice Macro Safety] Releasing {} held keys: {:?}",
            held_keys.len(),
            held_keys
        );
        for key in held_keys.iter().rev() {
            let _ = state.display_backend.send_key_up(app_handle, key).await;
        }
    }

    result
}

async fn execute_macro_steps_inner(
    app_handle: &AppHandle,
    steps: &[crate::config::MacroStep],
    held_keys: &mut Vec<String>,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    for (index, step) in steps.iter().enumerate() {
        match step {
            crate::config::MacroStep::KeyPress { key, hold_ms } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] KeyPress: '{}' (hold: {}ms)",
                    index + 1,
                    steps.len(),
                    key,
                    hold_ms
                );
                state
                    .display_backend
                    .send_key_combination(app_handle, key, *hold_ms)
                    .await?;
            }
            crate::config::MacroStep::KeyDown { key } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] KeyDown: '{}'",
                    index + 1,
                    steps.len(),
                    key
                );
                state.display_backend.send_key_down(app_handle, key).await?;
                if !held_keys.contains(key) {
                    held_keys.push(key.clone());
                }
            }
            crate::config::MacroStep::KeyUp { key } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] KeyUp: '{}'",
                    index + 1,
                    steps.len(),
                    key
                );
                state.display_backend.send_key_up(app_handle, key).await?;
                held_keys.retain(|k| k != key);
            }
            crate::config::MacroStep::Delay { duration_ms } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] Delay: {}ms",
                    index + 1,
                    steps.len(),
                    duration_ms
                );
                if *duration_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
                }
            }
            crate::config::MacroStep::TypeText { text } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] TypeText: '{}'",
                    index + 1,
                    steps.len(),
                    text
                );
                state
                    .display_backend
                    .type_text_hardware(app_handle, text, 0.005, 5)
                    .await?;
            }
        }
    }

    Ok(())
}

pub fn sync_voice_macro_listener(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let (enabled, has_macros) = {
        let config = state.config.lock().unwrap();
        (config.voice_macros_enabled, !config.voice_macros.is_empty())
    };

    let mut cancel_guard = state.voice_macro_cancel.lock().unwrap();

    if enabled && has_macros {
        if cancel_guard.is_none() {
            crate::log_info!("Starting Voice Macro background listener...");
            let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
            *cancel_guard = Some(cancel_tx);

            let app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                run_voice_macro_listener_loop(app, cancel_rx).await;
            });
        }
    } else if cancel_guard.is_some() {
        crate::log_info!("Stopping Voice Macro background listener...");
        if let Some(cancel_tx) = cancel_guard.take() {
            let _ = cancel_tx.send(());
        }
    }
}

struct SpeechChunk {
    samples: Vec<f32>,
    duration_secs: f32,
    peak_rms: f32,
    detected_at: Instant,
}

async fn run_voice_macro_listener_loop(
    app_handle: AppHandle,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let (audio_tx, audio_rx) = mpsc::sync_channel::<f32>(65536);
    let sample_rate;

    {
        let state = app_handle.state::<AppState>();
        let mut audio_guard = state.audio_engine.lock().unwrap();
        match audio_guard.as_mut() {
            Some(engine) => {
                sample_rate = engine.sample_rate;
                *engine.macro_tx.lock().unwrap() = Some(audio_tx);
            }
            None => {
                crate::log_warn!(
                    "Voice Macro listener: Audio engine not available, exiting listener loop"
                );
                return;
            }
        }
    }

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_drain = is_running.clone();

    // Single-slot bounded channel to prevent queue backlog
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<SpeechChunk>(1);
    let app_handle_for_vad = app_handle.clone();

    std::thread::spawn(move || {
        let frame_size = (sample_rate as f32 * 0.03) as usize; // 30ms window
        let pre_roll_frames = 7; // ~210ms pre-roll to catch speech onset
        let mut pre_roll_buffer: VecDeque<Vec<f32>> = VecDeque::with_capacity(pre_roll_frames);
        let mut frame_buffer = Vec::with_capacity(frame_size);
        let mut speech_buffer = Vec::with_capacity(sample_rate as usize * 4);
        let mut vad = crate::audio::vad::VoiceActivityDetector::new();
        let mut max_rms_in_phrase = 0.0f32;
        let max_speech_samples = (sample_rate as f32 * 4.0) as usize; // max 4.0s
        let min_speech_samples = (sample_rate as f32 * 0.30) as usize; // min 0.30s

        while is_running_drain.load(Ordering::Relaxed) {
            match audio_rx.recv_timeout(Duration::from_millis(30)) {
                Ok(sample) => {
                    frame_buffer.push(sample);
                    if frame_buffer.len() >= frame_size {
                        let open_threshold = {
                            let state = app_handle_for_vad.state::<AppState>();
                            let config = state.config.lock().unwrap();
                            config.voice_macro_activation_threshold
                        };

                        let vad_res = vad.process_frame(&frame_buffer, open_threshold);

                        if vad_res.speech_just_started {
                            speech_buffer.clear();
                            for pre in &pre_roll_buffer {
                                speech_buffer.extend_from_slice(pre);
                            }
                            speech_buffer.extend_from_slice(&frame_buffer);
                            max_rms_in_phrase = vad_res.smoothed_rms;
                            crate::log_info!(
                                "[Voice Macro VAD] Speech onset detected (RMS={:.4} >= threshold={:.4})",
                                vad_res.smoothed_rms,
                                open_threshold
                            );
                        } else if vad_res.is_speaking {
                            speech_buffer.extend_from_slice(&frame_buffer);
                            if vad_res.smoothed_rms > max_rms_in_phrase {
                                max_rms_in_phrase = vad_res.smoothed_rms;
                            }

                            if speech_buffer.len() >= max_speech_samples {
                                vad.reset();
                                let dur = speech_buffer.len() as f32 / sample_rate as f32;
                                crate::log_info!(
                                    "[Voice Macro VAD] Max duration reached, phrase finalized (dur={:.2}s, peak_rms={:.4})",
                                    dur,
                                    max_rms_in_phrase
                                );
                                let chunk = SpeechChunk {
                                    samples: speech_buffer.clone(),
                                    duration_secs: dur,
                                    peak_rms: max_rms_in_phrase,
                                    detected_at: Instant::now(),
                                };
                                let _ = chunk_tx.try_send(chunk);
                                max_rms_in_phrase = 0.0;
                                speech_buffer.clear();
                            }
                        } else if vad_res.speech_just_ended {
                            speech_buffer.extend_from_slice(&frame_buffer);
                            if speech_buffer.len() >= min_speech_samples {
                                let dur = speech_buffer.len() as f32 / sample_rate as f32;
                                crate::log_info!(
                                    "[Voice Macro VAD] Phrase finalized (duration={:.2}s, peak_rms={:.4}, samples={})",
                                    dur,
                                    max_rms_in_phrase,
                                    speech_buffer.len()
                                );

                                let chunk = SpeechChunk {
                                    samples: speech_buffer.clone(),
                                    duration_secs: dur,
                                    peak_rms: max_rms_in_phrase,
                                    detected_at: Instant::now(),
                                };
                                let _ = chunk_tx.try_send(chunk);
                            }
                            max_rms_in_phrase = 0.0;
                            speech_buffer.clear();
                        }

                        if pre_roll_buffer.len() >= pre_roll_frames {
                            pre_roll_buffer.pop_front();
                        }
                        pre_roll_buffer.push_back(frame_buffer.clone());
                        frame_buffer.clear();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });

    loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                crate::log_info!("Voice Macro listener received cancel signal");
                break;
            }
            Some(chunk) = chunk_rx.recv() => {
                let queue_age = chunk.detected_at.elapsed();
                if queue_age > Duration::from_millis(1500) {
                    crate::log_warn!(
                        "[Voice Macro] Discarding stale audio chunk (age={:.1}ms)",
                        queue_age.as_secs_f64() * 1000.0
                    );
                    continue;
                }

                crate::log_info!(
                    "[Voice Macro] Processing speech chunk (duration={:.2}s, peak_rms={:.4}, queue_age={:.1}ms)",
                    chunk.duration_secs,
                    chunk.peak_rms,
                    queue_age.as_secs_f64() * 1000.0
                );

                let state = app_handle.state::<AppState>();
                let is_idle = {
                    let session = state.session_state.lock().unwrap();
                    *session == SessionState::Idle
                };

                if !is_idle {
                    continue;
                }

                let (enabled, trigger_word, commands, sound_feedback, playback_dev) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.voice_macros_enabled,
                        config.voice_macro_trigger_word.clone(),
                        config.voice_macros.clone(),
                        config.voice_macro_sound_feedback,
                        config.playback_device.clone(),
                    )
                };

                if !enabled || commands.is_empty() {
                    continue;
                }

                let wav_bytes = match finalize_captured_audio_for_whisper(&chunk.samples, sample_rate) {
                    Ok(b) => b,
                    Err(e) => {
                        crate::log_warn!("Voice Macro audio conversion failed: {}", e);
                        continue;
                    }
                };

                let config_snapshot = {
                    let config = state.config.lock().unwrap();
                    config.clone()
                };

                let service = match state.engine_factory.create_service(&config_snapshot).await {
                    Ok(s) => s,
                    Err(e) => {
                        crate::log_warn!("Voice Macro transcription service creation failed: {}", e);
                        continue;
                    }
                };

                let lang = if config_snapshot.language == "auto" {
                    None
                } else {
                    Some(config_snapshot.language.as_str())
                };

                let prompt_hint = resolve_voice_macro_prompt_hint(&config_snapshot);
                let transcribe_start = Instant::now();
                match service.transcribe(&wav_bytes, lang, prompt_hint.as_deref()).await {
                    Ok(transcript) => {
                        let transcribe_time = transcribe_start.elapsed();
                        let trimmed = transcript.trim();
                        crate::log_info!(
                            "[Voice Macro] Transcription completed in {:.1}ms: \"{}\"",
                            transcribe_time.as_secs_f64() * 1000.0,
                            trimmed
                        );

                        if !trimmed.is_empty() {
                            let match_start = Instant::now();
                            let match_res = find_best_match(trimmed, &trigger_word, &commands);
                            if match_res.matched {
                                if let Some(matched_cmd) = match_res.matched_command {
                                    let match_time = match_start.elapsed();
                                    let steps = matched_cmd.resolve_steps();
                                    crate::log_info!(
                                        "[Voice Macro] MATCHED in {:.2}ms ({:.1}% match): \"{}\" -> \"{}\" ({} steps)",
                                        match_time.as_secs_f64() * 1000.0,
                                        match_res.similarity * 100.0,
                                        trimmed,
                                        matched_cmd.phrase,
                                        steps.len()
                                    );

                                    if sound_feedback {
                                        play_macro_trigger_sound(playback_dev);
                                    }

                                    let exec_start = Instant::now();
                                    if let Err(e) = execute_macro_command(&app_handle, &matched_cmd).await {
                                        crate::log_warn!("Voice Macro execution failed: {}", e);
                                    } else {
                                        let exec_time = exec_start.elapsed();
                                        let total_turnaround = chunk.detected_at.elapsed();
                                        crate::log_info!(
                                            "[Voice Macro] ⚡ Executed in {:.1}ms (Total speech-end to key turnaround: {:.1}ms)",
                                            exec_time.as_secs_f64() * 1000.0,
                                            total_turnaround.as_secs_f64() * 1000.0
                                        );
                                    }
                                }
                            } else {
                                crate::log_info!(
                                    "[Voice Macro] No macro matched for phrase: \"{}\" (best similarity: {:.1}%)",
                                    trimmed,
                                    match_res.similarity * 100.0
                                );
                            }
                        }
                    }
                    Err(e) => {
                        crate::log_warn!("Voice Macro background transcription error: {}", e);
                    }
                }
            }
        }
    }

    is_running.store(false, Ordering::Relaxed);
    {
        let state = app_handle.state::<AppState>();
        let mut audio_engine = state.audio_engine.lock().unwrap();
        if let Some(engine) = audio_engine.as_mut() {
            *engine.macro_tx.lock().unwrap() = None;
        }
    }
    crate::log_info!("Voice Macro listener loop terminated");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_phrase() {
        assert_eq!(normalize_phrase("Airstrike, please!"), "airstrike please");
        assert_eq!(normalize_phrase("  Drop   Smoke ... "), "drop smoke");
    }

    #[test]
    fn test_match_phrase_direct() {
        let commands = vec![
            VoiceMacroCommand {
                id: "1".into(),
                phrase: "airstrike".into(),
                steps: vec![crate::config::MacroStep::KeyPress {
                    key: "F3".into(),
                    hold_ms: 50,
                }],
                key_combination: None,
                hold_ms: None,
                delay_after_ms: None,
            },
            VoiceMacroCommand {
                id: "2".into(),
                phrase: "drop smoke".into(),
                steps: vec![crate::config::MacroStep::KeyPress {
                    key: "Ctrl+2".into(),
                    hold_ms: 50,
                }],
                key_combination: None,
                hold_ms: None,
                delay_after_ms: None,
            },
        ];

        let matched = match_phrase("airstrike", "", &commands);
        assert!(matched.is_some());
        let steps = matched.unwrap().resolve_steps();
        assert_eq!(
            steps,
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50
            }]
        );

        let matched_smoke = match_phrase("let's drop smoke now", "", &commands);
        assert!(matched_smoke.is_some());
        let steps_smoke = matched_smoke.unwrap().resolve_steps();
        assert_eq!(
            steps_smoke,
            vec![crate::config::MacroStep::KeyPress {
                key: "Ctrl+2".into(),
                hold_ms: 50
            }]
        );

        let no_match = match_phrase("call artillery", "", &commands);
        assert!(no_match.is_none());
    }

    #[test]
    fn test_match_phrase_with_trigger_word() {
        let commands = vec![VoiceMacroCommand {
            id: "1".into(),
            phrase: "airstrike".into(),
            steps: vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
            key_combination: None,
            hold_ms: None,
            delay_after_ms: None,
        }];

        assert!(match_phrase("airstrike", "computer", &commands).is_none());

        let matched = match_phrase("computer airstrike", "computer", &commands);
        assert!(matched.is_some());
        let steps = matched.unwrap().resolve_steps();
        assert_eq!(
            steps,
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50
            }]
        );

        let matched_punct = match_phrase("Computer, airstrike!", "computer", &commands);
        assert!(matched_punct.is_some());
    }

    #[test]
    fn test_match_phrase_space_insensitive_and_compound_words() {
        let commands = vec![VoiceMacroCommand {
            id: "1".into(),
            phrase: "call airstrike".into(),
            steps: vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
            key_combination: None,
            hold_ms: None,
            delay_after_ms: None,
        }];

        // Transcribed with two words "air strike"
        let matched = match_phrase("call air strike", "", &commands);
        assert!(matched.is_some());

        // Transcribed as one compound word "callairstrike"
        let matched_collapsed = match_phrase("callairstrike", "", &commands);
        assert!(matched_collapsed.is_some());
    }

    #[test]
    fn test_match_phrase_phonetic_fuzzy_tolerance() {
        let commands = vec![VoiceMacroCommand {
            id: "1".into(),
            phrase: "call airstrike".into(),
            steps: vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
            key_combination: None,
            hold_ms: None,
            delay_after_ms: None,
        }];

        // Whisper homophone "Coal Air Strike." vs "call airstrike"
        let res = find_best_match("Coal Air Strike.", "", &commands);
        assert!(res.matched);
        assert!(res.similarity >= 0.78);
        assert_eq!(res.matched_command.unwrap().phrase, "call airstrike");
    }

    #[test]
    fn test_match_phrase_conversational_subsequence() {
        let commands = vec![VoiceMacroCommand {
            id: "1".into(),
            phrase: "call airstrike".into(),
            steps: vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
            key_combination: None,
            hold_ms: None,
            delay_after_ms: None,
        }];

        let res = find_best_match("Can we call an airstrike on that hill?", "", &commands);
        assert!(res.matched);
        assert_eq!(res.matched_command.unwrap().phrase, "call airstrike");
    }
}
