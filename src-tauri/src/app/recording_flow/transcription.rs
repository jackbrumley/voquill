//! Turns a finished recording into text: the whole recording at once, or,
//! when speaker diarization is on, one piece per speaker turn. A failed
//! transcription is recorded in history here, where its cause is known.

use super::audio_processing;
use crate::audio;
use crate::diarization::Segment;
use crate::transcription::TranscriptionService;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter, Manager};

type FlowResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Service = dyn TranscriptionService + Send + Sync;

/// What a history entry for this dictation needs to know.
pub(super) struct SessionContext<'a> {
    pub session_uuid: &'a str,
    pub saved_audio_file: Option<&'a str>,
    pub duration_secs: f64,
    pub lang_code_str: &'a str,
    pub prompt_name: Option<&'a str>,
    pub history_limit: usize,
}

pub(super) struct Transcript {
    pub text: String,
    /// One entry per speaker turn, only when more than one speaker spoke.
    pub speaker_segments: Vec<Segment>,
}

impl Transcript {
    fn plain(text: String) -> Self {
        Self {
            text,
            speaker_segments: Vec::new(),
        }
    }
}

/// Transcribes the recording. With `diarization_threshold`, it is split by
/// speaker first; if that fails or finds no speech, the whole recording is
/// transcribed instead.
pub(super) async fn transcribe_recording(
    service: &Service,
    audio_data: &[u8],
    lang_code: Option<&str>,
    prompt_hint: Option<&str>,
    diarization_threshold: Option<f32>,
    ctx: &SessionContext<'_>,
    app_handle: &AppHandle,
) -> FlowResult<Transcript> {
    if let Some(threshold) = diarization_threshold {
        if let Some(transcript) = transcribe_by_speaker(
            service,
            audio_data,
            lang_code,
            prompt_hint,
            threshold,
            app_handle,
        )
        .await
        {
            return Ok(transcript);
        }
    }
    transcribe_full_audio(service, audio_data, lang_code, prompt_hint, ctx, app_handle)
        .await
        .map(Transcript::plain)
}

/// Splits the recording by speaker and transcribes each turn. Returns `None`
/// when diarization is unavailable or finds no segments, so the caller falls
/// back to the whole recording.
async fn transcribe_by_speaker(
    service: &Service,
    audio_data: &[u8],
    lang_code: Option<&str>,
    prompt_hint: Option<&str>,
    cluster_threshold: f32,
    app_handle: &AppHandle,
) -> Option<Transcript> {
    let temp_dir = crate::paths::temp_dir();
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join(format!(
        "recording_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if let Err(e) = std::fs::write(&temp_path, audio_data) {
        crate::log_warn!("Failed to save temp audio for diarization: {}", e);
        return None;
    }

    let diar_result =
        audio_processing::run_diarization_for_recording(app_handle, &temp_path, cluster_threshold)
            .await;
    let _ = std::fs::remove_file(&temp_path);

    let diar = match diar_result {
        Ok(diar) if !diar.segments.is_empty() => diar,
        Ok(_) => {
            crate::log_warn!("Diarization returned 0 segments — transcribing full recording");
            return None;
        }
        Err(e) => {
            crate::log_warn!("Diarization failed, transcribing full recording: {}", e);
            return None;
        }
    };
    crate::log_info!(
        "Diarization returned {} segments from {}",
        diar.segments.len(),
        diar.provider
    );

    let mut segment_pairs: Vec<(Option<String>, String)> = Vec::new();
    for seg in &diar.segments {
        let start = seg.start_sec.unwrap_or(0.0);
        let end = seg.end_sec.unwrap_or(1000000.0);

        let seg_wav = match audio::extract_segment_wav(audio_data, start, end) {
            Ok(w) => w,
            Err(e) => {
                crate::log_warn!("Failed to extract segment, skipping: {}", e);
                continue;
            }
        };

        let seg_text = match service.transcribe(&seg_wav, lang_code, prompt_hint).await {
            Ok(t) => t,
            Err(e) => {
                crate::log_warn!("Segment transcription failed, skipping: {}", e);
                continue;
            }
        };
        crate::log_info!(
            "Segment [{}]: \"{}\"",
            seg.speaker.as_deref().unwrap_or("?"),
            seg_text
        );
        segment_pairs.push((seg.speaker.clone(), seg_text));
    }

    Some(join_speaker_turns(segment_pairs))
}

/// One speaker: plain text. Several: one `[Speaker] text` line per turn,
/// with the turns kept as segments for history.
fn join_speaker_turns(segment_pairs: Vec<(Option<String>, String)>) -> Transcript {
    let unique_speakers: HashSet<Option<&str>> =
        segment_pairs.iter().map(|(s, _)| s.as_deref()).collect();

    if unique_speakers.len() <= 1 {
        let text = segment_pairs
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        return Transcript::plain(text);
    }

    let text = segment_pairs
        .iter()
        .map(|(s, t)| format!("[{}] {}", s.as_deref().unwrap_or("Speaker"), t))
        .collect::<Vec<_>>()
        .join("\n");
    let speaker_segments = segment_pairs
        .into_iter()
        .map(|(speaker, text)| Segment {
            speaker,
            text,
            start_sec: None,
            end_sec: None,
        })
        .collect();
    Transcript {
        text,
        speaker_segments,
    }
}

async fn transcribe_full_audio(
    service: &Service,
    audio_data: &[u8],
    lang_code: Option<&str>,
    prompt_hint: Option<&str>,
    ctx: &SessionContext<'_>,
    app_handle: &AppHandle,
) -> FlowResult<String> {
    match service.transcribe(audio_data, lang_code, prompt_hint).await {
        Ok(text) => {
            crate::log_info!(
                "[session:{}] Transcription received ({}): \"{}\"",
                &ctx.session_uuid[..8],
                service.service_name(),
                text
            );
            Ok(text)
        }
        Err(error) => {
            crate::log_info!(
                "[session:{}] Transcription failed ({}): {}",
                &ctx.session_uuid[..8],
                service.service_name(),
                error
            );
            let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
                session_uuid: ctx.session_uuid,
                status: "failed",
                text: "",
                raw_text: None,
                error_message: Some(&format!("Transcription failed: {}", error)),
                segments: None,
                audio_file: ctx.saved_audio_file,
                duration_secs: Some(ctx.duration_secs),
                engine: Some(service.service_name()),
                source: Some("mic"),
                language: Some(ctx.lang_code_str),
                prompt_name: ctx.prompt_name,
                limit: Some(ctx.history_limit),
            });
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("history-updated", ());
            }
            Err(error.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_speaker_is_joined_as_plain_text() {
        let transcript = join_speaker_turns(vec![
            (Some("A".into()), "Hello".into()),
            (Some("A".into()), "there".into()),
        ]);
        assert_eq!(transcript.text, "Hello there");
        assert!(transcript.speaker_segments.is_empty());
    }

    #[test]
    fn several_speakers_get_one_labelled_line_each() {
        let transcript = join_speaker_turns(vec![
            (Some("A".into()), "Hi".into()),
            (Some("B".into()), "Hello".into()),
        ]);
        assert_eq!(transcript.text, "[A] Hi\n[B] Hello");
        assert_eq!(transcript.speaker_segments.len(), 2);
    }
}
