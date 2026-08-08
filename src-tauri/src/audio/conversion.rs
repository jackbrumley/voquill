use hound::{WavSpec, WavWriter};

pub fn convert_audio_for_whisper(
    data: &[u8],
    rate: u32,
    _chans: u16,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if rate == 16000 {
        return Ok(data.to_vec());
    }
    let mut reader = hound::WavReader::new(std::io::Cursor::new(data))?;
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap_or(0)).collect();
    let mut mono = if reader.spec().channels == 2 {
        samples
            .chunks(2)
            .map(|c| {
                if c.len() == 2 {
                    ((c[0] as i32 + c[1] as i32) / 2) as i16
                } else {
                    c[0]
                }
            })
            .collect()
    } else {
        samples
    };
    if reader.spec().sample_rate != 16000 {
        mono = resample_audio(&mono, reader.spec().sample_rate, 16000);
    }
    let mut out = Vec::new();
    {
        let mut w = WavWriter::new(
            std::io::Cursor::new(&mut out),
            WavSpec {
                channels: 1,
                sample_rate: 16000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )?;
        for s in mono {
            w.write_sample(s)?;
        }
        w.finalize()?;
    }
    Ok(out)
}

pub fn resample_audio(samples: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from == to {
        return samples.to_vec();
    }

    if from > to && from.is_multiple_of(to) {
        let ratio = (from / to) as usize;
        let mut out = Vec::with_capacity(samples.len() / ratio);
        for chunk in samples.chunks_exact(ratio) {
            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
            out.push((sum / ratio as i32) as i16);
        }
        return out;
    }

    let ratio = from as f64 / to as f64;
    let len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = pos - idx as f64;
        if idx + 1 < samples.len() {
            let s1 = samples[idx] as f64;
            let s2 = samples[idx + 1] as f64;
            out.push((s1 + (s2 - s1) * frac) as i16);
        } else if idx < samples.len() {
            out.push(samples[idx]);
        }
    }
    out
}

pub fn resample_audio_f32(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to {
        return samples.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = pos - idx as f64;
        if idx + 1 < samples.len() {
            let s1 = samples[idx] as f64;
            let s2 = samples[idx + 1] as f64;
            out.push((s1 + (s2 - s1) * frac) as f32);
        } else if idx < samples.len() {
            out.push(samples[idx]);
        }
    }
    out
}

pub fn process_sample(s: f32) -> i16 {
    let clipped = soft_clip(s);
    (clipped * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16
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
