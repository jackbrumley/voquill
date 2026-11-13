//! Linux: other applications' playback streams (PulseAudio sink inputs, also
//! served by PipeWire through pipewire-pulse), through the control
//! connection in `audio::pulse_server`.

use super::{Adjustment, LOWERED_VOLUME_FACTOR};
use crate::audio::pulse_server;
use pulseaudio::protocol::{ChannelVolume, Prop, SinkInputInfo, Volume};
use std::collections::HashMap;

pub struct AdjustedStreams(Vec<AdjustedStream>);

impl AdjustedStreams {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

struct AdjustedStream {
    index: u32,
    name: String,
    change: Change,
}

enum Change {
    Muted,
    /// Boxed: a `ChannelVolume` holds room for 32 channels.
    Lowered(Box<LoweredVolume>),
}

struct LoweredVolume {
    original: ChannelVolume,
    lowered: ChannelVolume,
}

pub fn adjust(adjustment: Adjustment) -> Result<AdjustedStreams, String> {
    let own_process_id = std::process::id();
    let mut adjusted = Vec::new();

    for stream in pulse_server::playback_streams()? {
        if belongs_to_process(&stream, own_process_id) || stream.muted {
            continue;
        }
        let name = stream_name(&stream);
        let change = match adjustment {
            Adjustment::Mute => {
                pulse_server::set_playback_stream_mute(stream.index, true).map(|()| Change::Muted)
            }
            Adjustment::Lower => {
                if !stream.has_volume || !stream.volume_writable {
                    continue;
                }
                let lowered = scaled(&stream.cvolume, LOWERED_VOLUME_FACTOR);
                pulse_server::set_playback_stream_volume(stream.index, lowered).map(|()| {
                    Change::Lowered(Box::new(LoweredVolume {
                        original: stream.cvolume,
                        lowered,
                    }))
                })
            }
        };
        match change {
            Ok(change) => adjusted.push(AdjustedStream {
                index: stream.index,
                name,
                change,
            }),
            Err(error) => crate::log_warn!("Skipped playback stream '{}': {}", name, error),
        }
    }

    Ok(AdjustedStreams(adjusted))
}

pub fn restore(adjusted: AdjustedStreams) -> Result<(), String> {
    let current: HashMap<u32, SinkInputInfo> = pulse_server::playback_streams()?
        .into_iter()
        .map(|stream| (stream.index, stream))
        .collect();

    for stream in adjusted.0 {
        let Some(now) = current.get(&stream.index) else {
            continue; // The application stopped playing.
        };
        let result = match stream.change {
            Change::Muted if now.muted => {
                pulse_server::set_playback_stream_mute(stream.index, false)
            }
            Change::Lowered(volume) if now.cvolume == volume.lowered => {
                pulse_server::set_playback_stream_volume(stream.index, volume.original)
            }
            _ => {
                crate::log_info!(
                    "Playback stream '{}' was changed during the recording; leaving it as set",
                    stream.name
                );
                Ok(())
            }
        };
        if let Err(error) = result {
            crate::log_warn!(
                "Could not restore playback stream '{}': {}",
                stream.name,
                error
            );
        }
    }
    Ok(())
}

fn belongs_to_process(stream: &SinkInputInfo, process_id: u32) -> bool {
    stream
        .props
        .get(Prop::ApplicationProcessId)
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.trim_end_matches('\0').parse::<u32>().ok())
        == Some(process_id)
}

fn stream_name(stream: &SinkInputInfo) -> String {
    stream
        .props
        .get(Prop::ApplicationName)
        .map(|value| {
            String::from_utf8_lossy(value)
                .trim_end_matches('\0')
                .to_string()
        })
        .unwrap_or_else(|| stream.name.to_string_lossy().into_owned())
}

fn scaled(volume: &ChannelVolume, factor: f32) -> ChannelVolume {
    let mut result = ChannelVolume::empty();
    for channel in volume.channels() {
        result.push(Volume::from_linear(channel.to_linear() * factor));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowering_scales_every_channel_by_the_factor() {
        let lowered = scaled(&ChannelVolume::norm(2), LOWERED_VOLUME_FACTOR);
        assert_eq!(lowered.channels().len(), 2);
        for channel in lowered.channels() {
            assert!((channel.to_linear() - LOWERED_VOLUME_FACTOR).abs() < 0.01);
        }
    }

    #[test]
    fn streams_are_matched_to_their_process() {
        let mut stream = SinkInputInfo::default();
        stream.props.set(
            Prop::ApplicationProcessId,
            std::ffi::CString::new("4242").unwrap(),
        );
        assert!(belongs_to_process(&stream, 4242));
        assert!(!belongs_to_process(&stream, 1));
        assert!(!belongs_to_process(&SinkInputInfo::default(), 4242));
    }
}
