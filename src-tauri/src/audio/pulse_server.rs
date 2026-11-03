//! Persistent PulseAudio protocol connection for the endpoint details cpal
//! does not expose.
//!
//! cpal's PulseAudio host opens a stream on a source's server index but only
//! reports the source *name* as the device ID. A device that is unplugged and
//! re-plugged keeps its name yet gets a new index, and PipeWire leaves a stream
//! whose source disappeared on the fallback device instead of moving it back.
//! The capture engine therefore records the index it was opened on, read here,
//! so a re-plugged device is recognised as a different endpoint.

use std::ffi::CString;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

/// How long to wait for the server handshake (it has no socket timeout).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

static CLIENT: Mutex<Option<pulseaudio::Client>> = Mutex::new(None);

/// The server index of the source named `source_name`.
pub fn source_index(source_name: &str) -> Result<u32, String> {
    let name = CString::new(source_name)
        .map_err(|_| format!("PulseAudio source name '{source_name}' contains a NUL byte"))?;
    let client = connected_client()?;

    futures::executor::block_on(client.lookup_source_by_name(name)).map_err(|error| {
        // Anything but a server-side answer means the connection is unusable;
        // drop it so the next query reconnects.
        if !matches!(error, pulseaudio::ClientError::ServerError(_)) {
            *CLIENT.lock().unwrap() = None;
        }
        format!("Failed to look up PulseAudio source '{source_name}': {error}")
    })
}

fn connected_client() -> Result<pulseaudio::Client, String> {
    let mut client_slot = CLIENT.lock().unwrap();
    if let Some(client) = client_slot.as_ref() {
        return Ok(client.clone());
    }

    // `Client::from_env` blocks on the auth handshake without a timeout, so it
    // runs on its own thread; a hung server costs one leaked thread, not a
    // frozen caller.
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let name = CString::new(format!("voquill-endpoints-{}", std::process::id()))
            .expect("client name has no NUL bytes");
        let _ = sender.send(pulseaudio::Client::from_env(name));
    });
    let client = receiver
        .recv_timeout(CONNECT_TIMEOUT)
        .map_err(|_| "PulseAudio server did not answer the connection handshake".to_string())?
        .map_err(|error| format!("Failed to connect to the PulseAudio server: {error}"))?;

    *client_slot = Some(client.clone());
    Ok(client)
}
