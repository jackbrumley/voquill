//! The process's persistent connections to the PulseAudio server (also served
//! by PipeWire through pipewire-pulse).
//!
//! Every connection is opened once and reused. The `pulseaudio` crate's
//! reactor thread owns the socket and only exits when the connection fails, so
//! dropping a healthy client (or a cpal host built on one) leaks the
//! connection. Opening one per query exhausts the server's client limit
//! (pipewire-pulse: 64), after which it refuses every application on the
//! desktop. A cached connection is replaced only once it has failed, when its
//! reactor has already exited and released the socket.
//!
//! Two connections are kept:
//! - the cpal host, through which devices are listed, resolved and opened;
//! - an endpoint client for the details cpal does not expose. cpal opens a
//!   stream on a source's server index but only reports the source *name* as
//!   the device ID. A device that is unplugged and re-plugged keeps its name
//!   yet gets a new index, and PipeWire leaves a stream whose source
//!   disappeared on the fallback device instead of moving it back. The capture
//!   engine therefore records the index it was opened on, read here, so a
//!   re-plugged device is recognised as a different endpoint.

use std::ffi::CString;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

/// How long to wait for the server handshake (it has no socket timeout).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

static HOST: Mutex<Option<Arc<cpal::Host>>> = Mutex::new(None);

static ENDPOINT_CLIENT: Mutex<Option<pulseaudio::Client>> = Mutex::new(None);

/// The shared cpal PulseAudio host, connecting on first use.
pub fn host() -> Result<Arc<cpal::Host>, String> {
    let mut host_slot = HOST.lock().unwrap();
    if let Some(host) = host_slot.as_ref() {
        return Ok(host.clone());
    }

    let host = cpal::host_from_id(cpal::HostId::PulseAudio)
        .map(Arc::new)
        .map_err(|error| format!("PulseAudio server is unavailable: {}", error))?;
    crate::log_info!("Connected the PulseAudio device host");
    *host_slot = Some(host.clone());
    Ok(host)
}

/// Drops the shared host if `error` shows its connection has failed, so the
/// next `host()` call reconnects. Any other error leaves the (still live)
/// connection in place.
pub fn discard_host_if_disconnected(error: &cpal::Error) {
    let connection_failed = matches!(
        error.kind(),
        cpal::ErrorKind::StreamInvalidated | cpal::ErrorKind::HostUnavailable
    );
    if connection_failed && HOST.lock().unwrap().take().is_some() {
        crate::log_warn!(
            "PulseAudio device host connection lost ({}); reconnecting on next use",
            error
        );
    }
}

/// The server index of the source named `source_name`.
pub fn source_index(source_name: &str) -> Result<u32, String> {
    let name = CString::new(source_name)
        .map_err(|_| format!("PulseAudio source name '{source_name}' contains a NUL byte"))?;
    let client = endpoint_client()?;

    futures::executor::block_on(client.lookup_source_by_name(name)).map_err(|error| {
        // Anything but a server-side answer means the connection has failed;
        // drop it so the next query reconnects.
        if !matches!(error, pulseaudio::ClientError::ServerError(_)) {
            *ENDPOINT_CLIENT.lock().unwrap() = None;
        }
        format!("Failed to look up PulseAudio source '{source_name}': {error}")
    })
}

fn endpoint_client() -> Result<pulseaudio::Client, String> {
    let mut client_slot = ENDPOINT_CLIENT.lock().unwrap();
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
