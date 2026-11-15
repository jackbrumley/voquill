//! Whether Voquill is set up enough to dictate, as decided by the readiness
//! gate (`src/readiness.ts`, the single source of truth) and reported here
//! by the frontend. Dictation and voice macros refuse to start until it says
//! ready, so pressing the hotkey during initial setup never records.

use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppReadiness {
    /// The frontend has not reported yet (still starting up).
    Unknown,
    /// Setup is incomplete; the text explains what is missing.
    NotReady(String),
    Ready,
}

pub struct ReadinessGate {
    current: Mutex<AppReadiness>,
}

impl Default for ReadinessGate {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadinessGate {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(AppReadiness::Unknown),
        }
    }

    pub fn report(&self, readiness: AppReadiness) {
        let mut current = self.current.lock().unwrap();
        if *current != readiness {
            crate::log_info!("App readiness: {:?} -> {:?}", *current, readiness);
            *current = readiness;
        }
    }

    /// `Ok` when dictation may start, otherwise why not.
    pub fn check(&self) -> Result<(), String> {
        match &*self.current.lock().unwrap() {
            AppReadiness::Ready => Ok(()),
            AppReadiness::Unknown => Err("Voquill is still starting up".to_string()),
            AppReadiness::NotReady(reason) => Err(format!("Setup is not complete: {}", reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_until_reported_ready() {
        let gate = ReadinessGate::new();
        assert!(gate.check().is_err());
        gate.report(AppReadiness::NotReady("no microphone".to_string()));
        assert!(gate.check().unwrap_err().contains("no microphone"));
        gate.report(AppReadiness::Ready);
        assert!(gate.check().is_ok());
    }
}
