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
//! Three connections are kept:
//! - the cpal host, through which devices are listed, resolved and opened;
//! - an endpoint client for the details cpal does not expose. cpal opens a
//!   stream on a source's server index but only reports the source *name* as
//!   the device ID. A device that is unplugged and re-plugged keeps its name
//!   yet gets a new index, and PipeWire leaves a stream whose source
//!   disappeared on the fallback device instead of moving it back. The capture
//!   engine therefore records the index it was opened on, read here, so a
//!   re-plugged device is recognised as a different endpoint;
//! - a control connection for other applications' playback streams (sink
//!   inputs), which the `pulseaudio` client does not expose. It speaks the
//!   crate's public protocol layer over a plain socket that we own, so
//!   dropping it closes it; it is still kept and reused like the others.

use pulseaudio::protocol;
use std::ffi::CString;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

/// How long to wait for the server handshake (it has no socket timeout).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

static HOST: Mutex<Option<Arc<cpal::Host>>> = Mutex::new(None);

static ENDPOINT_CLIENT: Mutex<Option<pulseaudio::Client>> = Mutex::new(None);

static CONTROL_CONNECTION: Mutex<Option<ControlConnection>> = Mutex::new(None);

/// How long one control request may wait for the server's reply.
const CONTROL_REPLY_TIMEOUT: Duration = Duration::from_secs(2);

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

/// Every application's playback stream (sink input), Voquill's own included.
pub fn playback_streams() -> Result<Vec<protocol::SinkInputInfo>, String> {
    with_control_connection("list playback streams", |connection| {
        connection.request::<protocol::SinkInputInfoList>(protocol::Command::GetSinkInputInfoList)
    })
}

pub fn set_playback_stream_volume(
    index: u32,
    volume: protocol::ChannelVolume,
) -> Result<(), String> {
    with_control_connection("set a playback stream's volume", |connection| {
        connection.request_ack(protocol::Command::SetSinkInputVolume(
            protocol::SetStreamVolumeParams { index, volume },
        ))
    })
}

pub fn set_playback_stream_mute(index: u32, mute: bool) -> Result<(), String> {
    with_control_connection("mute or unmute a playback stream", |connection| {
        connection.request_ack(protocol::Command::SetSinkInputMute(
            protocol::SetStreamMuteParams { index, mute },
        ))
    })
}

/// Runs `operation` on the shared control connection, connecting first if
/// needed. A failure other than the server rejecting the request means the
/// connection is broken, so it is dropped and the next call reconnects.
fn with_control_connection<T>(
    description: &str,
    operation: impl FnOnce(&mut ControlConnection) -> Result<T, protocol::ProtocolError>,
) -> Result<T, String> {
    let mut connection_slot = CONTROL_CONNECTION.lock().unwrap();
    if connection_slot.is_none() {
        *connection_slot = Some(ControlConnection::connect()?);
    }
    let connection = connection_slot.as_mut().expect("connection was just set");

    operation(connection).map_err(|error| {
        if !matches!(error, protocol::ProtocolError::ServerError(_)) {
            *connection_slot = None;
        }
        format!("Failed to {description} on the PulseAudio server: {error}")
    })
}

struct ControlConnection {
    socket: BufReader<UnixStream>,
    protocol_version: u16,
    next_sequence: u32,
}

impl ControlConnection {
    fn connect() -> Result<Self, String> {
        let socket_path = pulseaudio::socket_path_from_env()
            .ok_or_else(|| "PulseAudio server socket not found".to_string())?;
        let socket = UnixStream::connect(&socket_path).map_err(|error| {
            format!(
                "Failed to connect to the PulseAudio server at {}: {error}",
                socket_path.display()
            )
        })?;
        for timeout_result in [
            socket.set_read_timeout(Some(CONTROL_REPLY_TIMEOUT)),
            socket.set_write_timeout(Some(CONTROL_REPLY_TIMEOUT)),
        ] {
            timeout_result
                .map_err(|error| format!("Failed to set PulseAudio socket timeout: {error}"))?;
        }

        let mut connection = Self {
            socket: BufReader::new(socket),
            protocol_version: protocol::MAX_VERSION,
            next_sequence: 0,
        };
        connection
            .handshake()
            .map_err(|error| format!("PulseAudio control handshake failed: {error}"))?;
        crate::log_info!("Connected the PulseAudio control connection");
        Ok(connection)
    }

    fn handshake(&mut self) -> Result<(), protocol::ProtocolError> {
        let cookie = pulseaudio::cookie_path_from_env()
            .and_then(|path| std::fs::read(path).ok())
            .unwrap_or_default();
        let auth_reply =
            self.request::<protocol::AuthReply>(protocol::Command::Auth(protocol::AuthParams {
                version: protocol::MAX_VERSION,
                supports_shm: false,
                supports_memfd: false,
                cookie,
            }))?;
        self.protocol_version = protocol::MAX_VERSION.min(auth_reply.version);

        let mut props = protocol::Props::new();
        props.set(
            protocol::Prop::ApplicationName,
            CString::new(format!("voquill-control-{}", std::process::id()))
                .expect("client name has no NUL bytes"),
        );
        props.set(
            protocol::Prop::ApplicationProcessId,
            CString::new(std::process::id().to_string()).expect("a number has no NUL bytes"),
        );
        self.request::<protocol::SetClientNameReply>(protocol::Command::SetClientName(props))?;
        Ok(())
    }

    fn send(&mut self, command: protocol::Command) -> Result<u32, protocol::ProtocolError> {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        protocol::write_command_message(
            self.socket.get_mut(),
            sequence,
            &command,
            self.protocol_version,
        )?;
        Ok(sequence)
    }

    fn request<R: protocol::CommandReply>(
        &mut self,
        command: protocol::Command,
    ) -> Result<R, protocol::ProtocolError> {
        let sequence = self.send(command)?;
        let (reply_sequence, reply) =
            protocol::read_reply_message::<R>(&mut self.socket, self.protocol_version)?;
        expect_sequence(sequence, reply_sequence)?;
        Ok(reply)
    }

    fn request_ack(&mut self, command: protocol::Command) -> Result<(), protocol::ProtocolError> {
        let sequence = self.send(command)?;
        let reply_sequence = protocol::read_ack_message(&mut self.socket)?;
        expect_sequence(sequence, reply_sequence)
    }
}

fn expect_sequence(sent: u32, received: u32) -> Result<(), protocol::ProtocolError> {
    if sent == received {
        Ok(())
    } else {
        Err(protocol::ProtocolError::Invalid(format!(
            "reply for request {received} arrived while waiting for {sent}"
        )))
    }
}
