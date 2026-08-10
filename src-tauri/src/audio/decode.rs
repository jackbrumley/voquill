use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

pub fn decode_compressed_audio(
    data: &[u8],
) -> Result<DecodedAudio, Box<dyn std::error::Error + Send + Sync>> {
    let cursor = std::io::Cursor::new(data.to_vec());
    let media_source_stream = MediaSourceStream::new(Box::new(cursor), Default::default());

    let probed = symphonia::default::get_probe().format(
        &Hint::new(),
        media_source_stream,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("no audio track found in file")?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("unsupported audio codec: {}", e))?;

    // Container-level codec parameters often omit the sample rate and channel count (common for
    // AAC in MP4). The first decoded buffer carries the authoritative signal spec, so capture it
    // there instead.
    let mut sample_rate: Option<u32> = codec_params.sample_rate;
    let mut channels: Option<usize> = codec_params.channels.map(|channels| channels.count());

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder = symphonia::default::get_codecs()
                    .make(&codec_params, &DecoderOptions::default())?;
                continue;
            }
            Err(e) => return Err(Box::new(e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                sample_rate = sample_rate.or(Some(spec.rate));
                channels = channels.or(Some(spec.channels.count()));

                let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                sample_buffer.copy_interleaved_ref(decoded);
                samples.extend_from_slice(sample_buffer.samples());
            }
            Err(SymphoniaError::DecodeError(e)) => {
                log::warn!("Skipping undecodable audio packet: {}", e);
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder = symphonia::default::get_codecs()
                    .make(&codec_params, &DecoderOptions::default())?;
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    if samples.is_empty() {
        return Err("no audio samples could be decoded from file".into());
    }

    Ok(DecodedAudio {
        samples,
        sample_rate: sample_rate.ok_or("could not determine audio sample rate")?,
        channels: channels.ok_or("could not determine audio channel count")?,
    })
}
