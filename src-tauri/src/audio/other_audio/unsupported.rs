//! Platforms without a per-application volume API (macOS). The setting is
//! hidden there (`is_supported`), so this only runs for a config file
//! carried over from another platform.

use super::Adjustment;

pub struct AdjustedStreams;

impl AdjustedStreams {
    pub fn len(&self) -> usize {
        0
    }
}

pub fn adjust(_adjustment: Adjustment) -> Result<AdjustedStreams, String> {
    Err("this platform cannot change other applications' volume".to_string())
}

pub fn restore(_streams: AdjustedStreams) -> Result<(), String> {
    Ok(())
}
