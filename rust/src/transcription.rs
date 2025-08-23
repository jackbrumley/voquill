use reqwest::multipart;
use serde_json::Value;

pub async fn transcribe_audio(
    audio_data: &[u8],
    api_key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    
    let form = multipart::Form::new()
        .part(
            "file",
            multipart::Part::bytes(audio_data.to_vec())
                .file_name("audio.wav")
                .mime_str("audio/wav")?,
        )
        .text("model", "whisper-1");

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("OpenAI API error: {}", error_text).into());
    }

    let json: Value = response.json().await?;
    let text = json["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(text)
}

pub async fn test_api_key(api_key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    
    // Create a minimal test audio file (silence)
    let test_audio = create_test_audio();
    
    let form = multipart::Form::new()
        .part(
            "file",
            multipart::Part::bytes(test_audio)
                .file_name("test.wav")
                .mime_str("audio/wav")?,
        )
        .text("model", "whisper-1");

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    Ok(response.status().is_success())
}

fn create_test_audio() -> Vec<u8> {
    // Create a minimal WAV file with 1 second of silence
    let sample_rate = 16000u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let duration_samples = sample_rate; // 1 second
    
    let mut wav_data = Vec::new();
    
    // WAV header
    wav_data.extend_from_slice(b"RIFF");
    let file_size = 36 + duration_samples * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&(file_size - 8).to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");
    
    // Format chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // Audio format (PCM)
    wav_data.extend_from_slice(&channels.to_le_bytes());
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());
    
    // Data chunk
    wav_data.extend_from_slice(b"data");
    let data_size = duration_samples * channels as u32 * bits_per_sample as u32 / 8;
    wav_data.extend_from_slice(&data_size.to_le_bytes());
    
    // Silent audio data (zeros)
    for _ in 0..duration_samples {
        wav_data.extend_from_slice(&0i16.to_le_bytes());
    }
    
    wav_data
}
