//! Windows: other applications' audio sessions on every active playback
//! device (`IAudioSessionManager2` + `ISimpleAudioVolume`).
//!
//! COM interface pointers are tied to the thread that created them, so a
//! session is remembered by its instance identifier and found again when it
//! is restored.

use super::{Adjustment, LOWERED_VOLUME_FACTOR};
use windows::core::{Interface, GUID};
use windows::Win32::Foundation::{RPC_E_CHANGED_MODE, S_OK};
use windows::Win32::Media::Audio::{
    eRender, AudioSessionStateExpired, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED,
};

pub struct AdjustedStreams(Vec<AdjustedSession>);

impl AdjustedStreams {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

struct AdjustedSession {
    instance_id: String,
    change: Change,
}

enum Change {
    Muted,
    Lowered { original: f32, lowered: f32 },
}

/// One application audio session, other than Voquill's own and the system
/// sounds session.
struct Session {
    instance_id: String,
    volume: ISimpleAudioVolume,
}

pub fn adjust(adjustment: Adjustment) -> Result<AdjustedStreams, String> {
    with_com(|| {
        let mut adjusted = Vec::new();
        for session in other_sessions()? {
            match adjust_session(&session, adjustment) {
                Ok(Some(change)) => adjusted.push(AdjustedSession {
                    instance_id: session.instance_id,
                    change,
                }),
                Ok(None) => {}
                Err(error) => {
                    crate::log_warn!("Skipped audio session '{}': {}", session.instance_id, error)
                }
            }
        }
        Ok(AdjustedStreams(adjusted))
    })
}

pub fn restore(adjusted: AdjustedStreams) -> Result<(), String> {
    with_com(|| {
        let sessions = other_sessions()?;
        for adjusted_session in adjusted.0 {
            let Some(session) = sessions
                .iter()
                .find(|session| session.instance_id == adjusted_session.instance_id)
            else {
                continue; // The application stopped playing.
            };
            if let Err(error) = restore_session(session, &adjusted_session.change) {
                crate::log_warn!(
                    "Could not restore audio session '{}': {}",
                    session.instance_id,
                    error
                );
            }
        }
        Ok(())
    })
}

fn adjust_session(
    session: &Session,
    adjustment: Adjustment,
) -> windows::core::Result<Option<Change>> {
    // SAFETY: plain COM calls on an interface obtained on this thread.
    unsafe {
        if session.volume.GetMute()?.as_bool() {
            return Ok(None); // Already muted by the user; leave it alone.
        }
        match adjustment {
            Adjustment::Mute => {
                session.volume.SetMute(true, &GUID::zeroed())?;
                Ok(Some(Change::Muted))
            }
            Adjustment::Lower => {
                let original = session.volume.GetMasterVolume()?;
                let lowered = original * LOWERED_VOLUME_FACTOR;
                session.volume.SetMasterVolume(lowered, &GUID::zeroed())?;
                Ok(Some(Change::Lowered { original, lowered }))
            }
        }
    }
}

fn restore_session(session: &Session, change: &Change) -> windows::core::Result<()> {
    // SAFETY: plain COM calls on an interface obtained on this thread.
    unsafe {
        match change {
            Change::Muted if session.volume.GetMute()?.as_bool() => {
                session.volume.SetMute(false, &GUID::zeroed())
            }
            Change::Lowered { original, lowered }
                if session.volume.GetMasterVolume()? == *lowered =>
            {
                session.volume.SetMasterVolume(*original, &GUID::zeroed())
            }
            _ => {
                crate::log_info!(
                    "Audio session '{}' was changed during the recording; leaving it as set",
                    session.instance_id
                );
                Ok(())
            }
        }
    }
}

/// Every live audio session on every active playback device, except
/// Voquill's own and the system sounds session.
fn other_sessions() -> Result<Vec<Session>, String> {
    let own_process_id = std::process::id();
    let mut sessions = Vec::new();

    // SAFETY: COM is initialized on this thread by `with_com`.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|error| format!("Failed to create MMDeviceEnumerator: {error}"))?;
        let devices = enumerator
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|error| format!("Failed to list playback devices: {error}"))?;
        let device_count = devices
            .GetCount()
            .map_err(|error| format!("Failed to count playback devices: {error}"))?;

        for device_index in 0..device_count {
            let Ok(device) = devices.Item(device_index) else {
                continue;
            };
            let manager: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
                Ok(manager) => manager,
                Err(error) => {
                    crate::log_warn!("Failed to open a playback device's sessions: {}", error);
                    continue;
                }
            };
            let Ok(session_list) = manager.GetSessionEnumerator() else {
                continue;
            };
            let session_count = session_list.GetCount().unwrap_or(0);
            for session_index in 0..session_count {
                let Ok(control) = session_list.GetSession(session_index) else {
                    continue;
                };
                if control
                    .GetState()
                    .is_ok_and(|state| state == AudioSessionStateExpired)
                {
                    continue;
                }
                let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                    continue;
                };
                if control2.IsSystemSoundsSession() == S_OK
                    || control2.GetProcessId().is_ok_and(|id| id == own_process_id)
                {
                    continue;
                }
                let (Some(instance_id), Ok(volume)) = (
                    session_instance_id(&control2),
                    control.cast::<ISimpleAudioVolume>(),
                ) else {
                    continue;
                };
                sessions.push(Session {
                    instance_id,
                    volume,
                });
            }
        }
    }
    Ok(sessions)
}

unsafe fn session_instance_id(control: &IAudioSessionControl2) -> Option<String> {
    let identifier = control.GetSessionInstanceIdentifier().ok()?;
    let text = identifier.to_string().ok();
    CoTaskMemFree(Some(identifier.0 as *const _));
    text
}

/// Runs `operation` with COM initialized on this thread, uninitializing it
/// afterwards if this call initialized it.
fn with_com<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    // SAFETY: balanced with CoUninitialize below when it succeeds.
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
        return Err(format!("Failed to initialize COM: {initialized:?}"));
    }
    let result = operation();
    if initialized.is_ok() {
        // SAFETY: this thread's successful CoInitializeEx above.
        unsafe { CoUninitialize() };
    }
    result
}
