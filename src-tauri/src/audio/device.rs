use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(target_os = "windows")]
use windows::Win32::Devices::FunctionDiscovery::{
    PKEY_Device_DeviceDesc, PKEY_Device_FriendlyName,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::PROPERTYKEY;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    eCapture, eRender, EDataFlow, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    STGM_READ,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;

#[derive(Serialize, Clone, Debug)]
pub struct AudioDevice {
    pub id: String,
    pub label: String,
    /// True only for the synthetic "System Default" entry injected by
    /// `list_devices`; real enumerated hardware always reports false.
    pub is_system_default: bool,
}

#[cfg(target_os = "windows")]
unsafe fn get_string_property(props: &IPropertyStore, key: *const PROPERTYKEY) -> Option<String> {
    let mut pv = match props.GetValue(key) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let result = match PropVariantToStringAlloc(&pv) {
        Ok(pwstr) => {
            if pwstr.is_null() {
                None
            } else {
                let s = pwstr.to_string().ok();
                CoTaskMemFree(Some(pwstr.0 as *const _));
                s
            }
        }
        Err(_) => None,
    };

    let _ = PropVariantClear(&mut pv);
    result.filter(|s| !s.trim().is_empty())
}

/// Active WASAPI endpoints for `data_flow` (`eCapture` or `eRender`), labelled
/// from their friendly name and description.
#[cfg(target_os = "windows")]
fn get_windows_audio_devices(data_flow: EDataFlow) -> Result<Vec<AudioDevice>, String> {
    let flow_label = if data_flow == eCapture {
        "capture"
    } else {
        "render"
    };
    let mut devices = Vec::new();
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() && hr.0 != 0x00040101 {
            crate::log_info!("Windows Audio: CoInitializeEx failed: {:?}", hr);
        }

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create MMDeviceEnumerator: {}", e))?;

        let collection = enumerator
            .EnumAudioEndpoints(data_flow, DEVICE_STATE_ACTIVE)
            .map_err(|e| format!("Failed to enum audio endpoints: {}", e))?;

        let count = collection
            .GetCount()
            .map_err(|e| format!("Failed to get device count: {}", e))?;

        crate::log_info!(
            "Windows Audio: Found {} active {} endpoints",
            count,
            flow_label
        );

        for i in 0..count {
            if let Ok(device) = collection.Item(i) {
                let id_pwstr = match device.GetId() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let id = id_pwstr.to_string().unwrap_or_default();
                CoTaskMemFree(Some(id_pwstr.0 as *const _));

                if let Ok(props) = device.OpenPropertyStore(STGM_READ) {
                    let friendly_name = get_string_property(&props, &PKEY_Device_FriendlyName);
                    let device_desc = get_string_property(&props, &PKEY_Device_DeviceDesc);

                    if friendly_name.is_none() && device_desc.is_none() {
                        if let Ok(p_count) = props.GetCount() {
                            crate::log_info!("Windows Audio: Store for {} has {} properties but FriendlyName/Desc missing", id, p_count);
                            for j in 0..p_count {
                                let mut pk = PROPERTYKEY::default();

                                if props.GetAt(j, &mut pk).is_ok() {
                                    crate::log_info!(
                                        "   Property {}: GUID={:?}, PID={}",
                                        j,
                                        pk.fmtid,
                                        pk.pid
                                    );
                                }
                            }
                        }
                    }

                    let friendly = friendly_name.unwrap_or_else(|| "Unknown Device".to_string());
                    let label = if let Some(desc) = device_desc {
                        let f_lower = friendly.to_lowercase();
                        let d_lower = desc.to_lowercase();

                        if f_lower.contains(&d_lower) {
                            friendly
                        } else if d_lower.contains(&f_lower) {
                            desc
                        } else {
                            format!("{} - {}", friendly, desc)
                        }
                    } else if friendly == "Unknown Device" {
                        format!("Unknown Device ({})", id)
                    } else {
                        friendly
                    };

                    crate::log_info!("Windows Audio: Enumerated '{}'", label);
                    devices.push(AudioDevice {
                        id,
                        label,
                        is_system_default: false,
                    });
                }
            }
        }
    }
    Ok(devices)
}

/// Prefix marking a device ID as a PulseAudio source or sink name
/// (`pulse:<name>`). These IDs are persisted in `config.json`, so the format
/// is stable across releases.
const PULSE_ID_PREFIX: &str = "pulse:";

/// ID of the synthetic "System Default" entry; also the persisted value
/// meaning "follow the audio server's default device".
const SYSTEM_DEFAULT_ID: &str = "default";

#[derive(Clone, Copy)]
enum DeviceDirection {
    Input,
    Output,
}

impl DeviceDirection {
    fn label(self) -> &'static str {
        match self {
            DeviceDirection::Input => "input",
            DeviceDirection::Output => "output",
        }
    }

    fn host_devices(self, host: &cpal::Host) -> Result<Vec<cpal::Device>, String> {
        let devices = match self {
            DeviceDirection::Input => host.input_devices().map(|devices| devices.collect()),
            DeviceDirection::Output => host.output_devices().map(|devices| devices.collect()),
        };
        devices.map_err(|error| {
            #[cfg(target_os = "linux")]
            if host.id() == cpal::HostId::PulseAudio {
                super::pulse_server::discard_host_if_disconnected(&error);
            }
            format!(
                "Failed to enumerate {} devices on the {} host: {}",
                self.label(),
                host.id().name(),
                error
            )
        })
    }

    fn host_default_device(self, host: &cpal::Host) -> Option<cpal::Device> {
        match self {
            DeviceDirection::Input => host.default_input_device(),
            DeviceDirection::Output => host.default_output_device(),
        }
    }
}

pub fn get_input_devices() -> Result<Vec<AudioDevice>, String> {
    list_devices(DeviceDirection::Input)
}

pub fn get_output_devices() -> Result<Vec<AudioDevice>, String> {
    list_devices(DeviceDirection::Output)
}

/// A capture device resolved from the configured device ID, together with the
/// identity of the concrete endpoint it is bound to right now.
pub struct CaptureEndpoint {
    pub device: cpal::Device,
    /// Changes whenever the configured ID would open a different endpoint:
    /// "System Default" moving to another device, or a device re-plugged.
    pub identity: String,
}

/// Resolves the configured input device ID (`None` / `"default"` for the
/// system default) to the exact endpoint the user selected.
pub fn resolve_capture_endpoint(target_id: Option<&str>) -> Result<CaptureEndpoint, String> {
    let device = resolve_device(target_id, DeviceDirection::Input)?;
    let identity = capture_endpoint_identity(&device)?;
    Ok(CaptureEndpoint { device, identity })
}

/// Resolves the configured playback device ID (`None` / `"default"` for the
/// system default) to the exact device the user selected.
pub fn lookup_output_device(target_id: Option<String>) -> Result<cpal::Device, String> {
    resolve_device(target_id.as_deref(), DeviceDirection::Output)
}

fn list_devices(direction: DeviceDirection) -> Result<Vec<AudioDevice>, String> {
    let mut devices = list_platform_devices(direction);
    if devices.is_empty() {
        devices = list_default_host_devices(direction);
    }

    devices.sort_by(|a, b| a.label.cmp(&b.label));
    devices.insert(
        0,
        AudioDevice {
            id: SYSTEM_DEFAULT_ID.to_string(),
            label: "System Default".to_string(),
            is_system_default: true,
        },
    );
    Ok(devices)
}

/// Platform-native enumeration, which yields friendlier labels than cpal's
/// generic listing. An empty result falls through to the default host.
#[cfg(target_os = "linux")]
fn list_platform_devices(direction: DeviceDirection) -> Vec<AudioDevice> {
    match list_pulse_devices(direction) {
        Ok(devices) => devices,
        Err(error) => {
            crate::log_warn!(
                "PulseAudio {} device listing unavailable, using the default audio host: {}",
                direction.label(),
                error
            );
            Vec::new()
        }
    }
}

#[cfg(target_os = "windows")]
fn list_platform_devices(direction: DeviceDirection) -> Vec<AudioDevice> {
    let devices = match direction {
        DeviceDirection::Input => get_windows_audio_devices(eCapture),
        DeviceDirection::Output => get_windows_audio_devices(eRender),
    };
    devices.unwrap_or_else(|error| {
        crate::log_warn!(
            "Windows {} device listing failed, using the default audio host: {}",
            direction.label(),
            error
        );
        Vec::new()
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn list_platform_devices(_direction: DeviceDirection) -> Vec<AudioDevice> {
    Vec::new()
}

/// The host for devices not addressed by a platform-specific ID: on Linux the
/// shared PulseAudio connection, or ALSA when no Pulse server answers.
///
/// Never call `cpal::default_host()` on Linux: it opens a new PulseAudio
/// connection on every call, and those are never closed (see `pulse_server`).
#[cfg(target_os = "linux")]
fn default_host() -> Result<Arc<cpal::Host>, String> {
    super::pulse_server::host().or_else(|pulse_error| {
        cpal::host_from_id(cpal::HostId::Alsa)
            .map(Arc::new)
            .map_err(|alsa_error| {
                format!("No audio host available ({pulse_error}; ALSA: {alsa_error})")
            })
    })
}

#[cfg(not(target_os = "linux"))]
fn default_host() -> Result<Arc<cpal::Host>, String> {
    Ok(Arc::new(cpal::default_host()))
}

#[cfg(target_os = "linux")]
fn list_pulse_devices(direction: DeviceDirection) -> Result<Vec<AudioDevice>, String> {
    let host = super::pulse_server::host()?;
    let devices = direction
        .host_devices(&host)?
        .into_iter()
        .filter_map(|device| {
            let name = device.id().ok()?.id().to_string();
            let label = device
                .description()
                .map(|description| description.name().to_string())
                .unwrap_or_else(|_| name.clone());
            let is_monitor = name.to_lowercase().contains(".monitor")
                || label.to_lowercase().contains("monitor");
            if matches!(direction, DeviceDirection::Input) && is_monitor {
                return None;
            }
            Some(AudioDevice {
                id: format!("{PULSE_ID_PREFIX}{name}"),
                label,
                is_system_default: false,
            })
        })
        .collect();
    Ok(devices)
}

fn list_default_host_devices(direction: DeviceDirection) -> Vec<AudioDevice> {
    let host = match default_host() {
        Ok(host) => host,
        Err(error) => {
            crate::log_warn!("{}", error);
            return Vec::new();
        }
    };
    let devices = match direction.host_devices(&host) {
        Ok(devices) => devices,
        Err(error) => {
            crate::log_warn!("{}", error);
            return Vec::new();
        }
    };

    let mut seen_labels: HashMap<String, usize> = HashMap::new();
    devices
        .into_iter()
        .filter_map(|device| {
            let id = device.id().ok()?.id().to_string();
            #[cfg(target_os = "linux")]
            if !id.starts_with("default:") && id != "pulse" && id != SYSTEM_DEFAULT_ID {
                return None;
            }

            let mut label = device
                .description()
                .map(|description| description.name().to_string())
                .unwrap_or_else(|_| id.clone());
            let count = seen_labels.entry(label.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                label = format!("{} ({})", label, *count);
            }
            Some(AudioDevice {
                id,
                label,
                is_system_default: false,
            })
        })
        .collect()
}

/// Callback period requested from PulseAudio streams.
#[cfg(target_os = "linux")]
const PULSE_STREAM_PERIOD_MS: u32 = 20;

/// Buffer size to request for a stream on `device`.
///
/// PulseAudio sizes buffers server-side when left to its default: capture
/// arrives in ~1s fragments (the level meter lags and the tail of a dictation
/// is still in flight at stop), and a whole playback clip is accepted at once,
/// so the "finished" callback fires long before the audio is heard. A fixed
/// 20ms period keeps both live. Other hosts already default to short periods.
pub(super) fn stream_buffer_size(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
) -> cpal::BufferSize {
    #[cfg(target_os = "linux")]
    {
        let is_pulse_device = device
            .id()
            .map(|device_id| device_id.host() == cpal::HostId::PulseAudio)
            .unwrap_or(false);
        if is_pulse_device {
            let period_frames = config.sample_rate() * PULSE_STREAM_PERIOD_MS / 1000;
            return match *config.buffer_size() {
                cpal::SupportedBufferSize::Range { min, max } => {
                    cpal::BufferSize::Fixed(period_frames.clamp(min, max))
                }
                cpal::SupportedBufferSize::Unknown => cpal::BufferSize::Fixed(period_frames),
            };
        }
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (device, config);

    cpal::BufferSize::Default
}

/// `host:id`, plus the server index for PulseAudio sources (see
/// `pulse_server` for why the name alone cannot identify the endpoint).
fn capture_endpoint_identity(device: &cpal::Device) -> Result<String, String> {
    let device_id = device
        .id()
        .map_err(|error| format!("Input device has no stable ID: {}", error))?;
    let identity = format!("{}:{}", device_id.host().name(), device_id.id());

    #[cfg(target_os = "linux")]
    if device_id.host() == cpal::HostId::PulseAudio {
        let index = super::pulse_server::source_index(device_id.id())?;
        return Ok(format!("{identity}#{index}"));
    }

    Ok(identity)
}

fn resolve_device(
    target_id: Option<&str>,
    direction: DeviceDirection,
) -> Result<cpal::Device, String> {
    let Some(id) = target_id.filter(|id| *id != SYSTEM_DEFAULT_ID) else {
        return resolve_default_device(direction);
    };

    match id.strip_prefix(PULSE_ID_PREFIX) {
        Some(pulse_name) => resolve_pulse_device(pulse_name, direction),
        None => find_host_device(&*default_host()?, id, direction),
    }
}

fn resolve_default_device(direction: DeviceDirection) -> Result<cpal::Device, String> {
    let host = default_host()?;
    if let Some(device) = direction.host_default_device(&host) {
        return Ok(device);
    }

    // cpal discards the reason a default-device lookup failed. Enumerating
    // reports it, and replaces the PulseAudio connection if it has dropped.
    direction.host_devices(&host)?;
    Err(format!(
        "No default {} device available on the {} host",
        direction.label(),
        host.id().name()
    ))
}

#[cfg(target_os = "linux")]
fn resolve_pulse_device(name: &str, direction: DeviceDirection) -> Result<cpal::Device, String> {
    find_host_device(&*super::pulse_server::host()?, name, direction)
}

#[cfg(not(target_os = "linux"))]
fn resolve_pulse_device(name: &str, direction: DeviceDirection) -> Result<cpal::Device, String> {
    Err(format!(
        "PulseAudio {} device '{}' cannot be opened on this platform",
        direction.label(),
        name
    ))
}

fn find_host_device(
    host: &cpal::Host,
    id: &str,
    direction: DeviceDirection,
) -> Result<cpal::Device, String> {
    let devices = direction.host_devices(host)?;
    let device_ids: Vec<String> = devices
        .iter()
        .filter_map(|device| device.id().ok().map(|device_id| device_id.id().to_string()))
        .collect();

    devices
        .into_iter()
        .find(|device| {
            device
                .id()
                .map(|device_id| device_id.id() == id)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            format!(
                "{} device '{}' not found on the {} host. available=[{}]",
                direction.label(),
                id,
                host.id().name(),
                device_ids.join(", ")
            )
        })
}
