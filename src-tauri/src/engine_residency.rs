//! Whether the local speech engine is in memory, reported to the UI.
//!
//! `EngineFactory` owns the engine caches and therefore the truth about what
//! is loaded; it reports every change here. This module turns those reports
//! into one published state (`Unloaded` / `Loading` / `Loaded`), emits
//! `engine-residency-changed` whenever it changes, and remembers when the
//! engine was last used so the idle unload can tell how long it has sat
//! unused.

use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const RESIDENCY_CHANGED_EVENT: &str = "engine-residency-changed";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum EngineResidency {
    /// No engine is in memory; the next dictation loads it.
    Unloaded,
    /// An engine is being loaded.
    Loading,
    /// An engine is in memory and starts transcribing instantly.
    Loaded,
}

pub struct ResidencyTracker {
    published: Mutex<EngineResidency>,
    loads_in_progress: AtomicUsize,
    last_used: Mutex<Instant>,
    app_handle: OnceLock<AppHandle>,
}

impl Default for ResidencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResidencyTracker {
    pub fn new() -> Self {
        Self {
            published: Mutex::new(EngineResidency::Unloaded),
            loads_in_progress: AtomicUsize::new(0),
            last_used: Mutex::new(Instant::now()),
            app_handle: OnceLock::new(),
        }
    }

    /// Connects the tracker to the app so state changes reach the windows.
    pub fn attach_app_handle(&self, app_handle: AppHandle) {
        let _ = self.app_handle.set(app_handle);
    }

    pub fn begin_load(&self) {
        self.loads_in_progress.fetch_add(1, Ordering::SeqCst);
    }

    pub fn end_load(&self) {
        self.loads_in_progress.fetch_sub(1, Ordering::SeqCst);
        self.mark_used();
    }

    /// Combines the load counter with whether a cache holds an engine.
    pub fn resolve(&self, engine_in_memory: bool) -> EngineResidency {
        if self.loads_in_progress.load(Ordering::SeqCst) > 0 {
            EngineResidency::Loading
        } else if engine_in_memory {
            EngineResidency::Loaded
        } else {
            EngineResidency::Unloaded
        }
    }

    /// Records `residency` and tells the windows if it changed.
    pub fn publish(&self, residency: EngineResidency) {
        let previous = {
            let mut published = self.published.lock().unwrap();
            std::mem::replace(&mut *published, residency)
        };
        if previous == residency {
            return;
        }
        crate::log_info!("Speech engine residency: {:?} -> {:?}", previous, residency);
        if let Some(app_handle) = self.app_handle.get() {
            let _ = app_handle.emit(RESIDENCY_CHANGED_EVENT, residency);
        }
    }

    /// Restarts the idle clock: the engine was just used or loaded.
    pub fn mark_used(&self) {
        *self.last_used.lock().unwrap() = Instant::now();
    }

    pub fn idle_for(&self) -> Duration {
        self.last_used.lock().unwrap().elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_reports_loading_while_a_load_runs() {
        let tracker = ResidencyTracker::new();
        tracker.begin_load();
        assert_eq!(tracker.resolve(false), EngineResidency::Loading);
        assert_eq!(tracker.resolve(true), EngineResidency::Loading);
        tracker.end_load();
        assert_eq!(tracker.resolve(true), EngineResidency::Loaded);
        assert_eq!(tracker.resolve(false), EngineResidency::Unloaded);
    }

    #[test]
    fn end_load_restarts_the_idle_clock() {
        let tracker = ResidencyTracker::new();
        *tracker.last_used.lock().unwrap() = Instant::now() - Duration::from_secs(600);
        tracker.begin_load();
        tracker.end_load();
        assert!(tracker.idle_for() < Duration::from_secs(5));
    }
}
