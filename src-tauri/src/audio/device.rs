use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use pulsectl::controllers::DeviceControl;

#[cfg(target_os = "windows")]
use windows::Win32::Devices::FunctionDiscovery::{
    PKEY_Device_DeviceDesc, PKEY_Device_FriendlyName,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::PROPERTYKEY;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    eCapture, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
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

#[cfg(target_os = "windows")]
fn get_windows_audio_devices() -> Result<Vec<AudioDevice>, String> {
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
            .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
            .map_err(|e| format!("Failed to enum audio endpoints: {}", e))?;

        let count = collection
            .GetCount()
            .map_err(|e| format!("Failed to get device count: {}", e))?;

        crate::log_info!("Windows Audio: Found {} active capture endpoints", count);

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
                    devices.push(AudioDevice { id, label });
                }
            }
        }
    }
    Ok(devices)
}

#[cfg(target_os = "linux")]
fn get_linux_pulse_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    let mut handler = pulsectl::controllers::SourceController::create()
        .map_err(|e| format!("Failed to connect to PulseAudio: {}", e))?;
    let sources = handler
        .list_devices()
        .map_err(|e| format!("Failed to list PulseAudio sources: {}", e))?;

    for source in sources {
        let name = source.name.clone().unwrap_or_default();
        let description = source.description.clone().unwrap_or_default();
        if name.to_lowercase().contains(".monitor")
            || description.to_lowercase().contains("monitor")
        {
            continue;
        }
        devices.push(AudioDevice {
            id: format!("pulse:{}", name),
            label: description,
        });
    }
    Ok(devices)
}

pub fn get_input_devices() -> Result<Vec<AudioDevice>, String> {
    let mut final_devices = Vec::new();
    #[cfg(target_os = "linux")]
    {
        if let Ok(devices) = get_linux_pulse_devices() {
            final_devices = devices;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(devices) = get_windows_audio_devices() {
            final_devices = devices;
        }
    }

    if final_devices.is_empty() {
        let mut seen_labels = HashMap::new();
        for host_id in cpal::available_hosts() {
            if let Ok(host) = cpal::host_from_id(host_id) {
                if let Ok(devices) = host.input_devices() {
                    for dev in devices {
                        let id = match dev.id() {
                            Ok(id) => id.1,
                            Err(_) => continue,
                        };
                        #[cfg(target_os = "linux")]
                        if !id.starts_with("default:") && id != "pulse" && id != "default" {
                            continue;
                        }

                        let mut label = match dev.description() {
                            Ok(desc) => desc.name().to_string(),
                            Err(_) => id.clone(),
                        };

                        let count = seen_labels.entry(label.clone()).or_insert(0);
                        *count += 1;
                        if *count > 1 {
                            label = format!("{} ({})", label, *count);
                        }
                        final_devices.push(AudioDevice { id, label });
                    }
                }
            }
        }
    }

    final_devices.sort_by(|a, b| a.label.cmp(&b.label));
    final_devices.insert(
        0,
        AudioDevice {
            id: "default".to_string(),
            label: "System Default".to_string(),
        },
    );
    Ok(final_devices)
}

pub fn lookup_device(target_id: Option<String>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    let target = target_id.filter(|id| id != "default");

    fn summarize_input_devices(host: &cpal::Host) -> String {
        match host.input_devices() {
            Ok(devices) => devices
                .map(|device| {
                    let identifier = device
                        .id()
                        .map(|id| id.1)
                        .unwrap_or_else(|_| "<unknown-id>".to_string());
                    let label = device
                        .description()
                        .map(|description| description.name().to_string())
                        .unwrap_or_else(|_| "<unknown-name>".to_string());
                    format!("{} ({})", identifier, label)
                })
                .collect::<Vec<String>>()
                .join(", "),
            Err(error) => format!("<failed to enumerate input devices: {}>", error),
        }
    }

    #[cfg(target_os = "linux")]
    fn summarize_pulse_sources() -> String {
        let mut controller = match pulsectl::controllers::SourceController::create() {
            Ok(controller) => controller,
            Err(error) => return format!("<failed to connect PulseAudio: {}>", error),
        };

        match controller.list_devices() {
            Ok(sources) => sources
                .into_iter()
                .map(|source| {
                    source
                        .name
                        .unwrap_or_else(|| "<unknown-source>".to_string())
                })
                .collect::<Vec<String>>()
                .join(", "),
            Err(error) => format!("<failed to list PulseAudio sources: {}>", error),
        }
    }

    let available_inputs = summarize_input_devices(&host);

    if let Some(name) = target {
        #[cfg(target_os = "linux")]
        if let Some(stripped) = name.strip_prefix("pulse:") {
            std::env::set_var("PULSE_SOURCE", stripped);
            return host.default_input_device().ok_or_else(|| {
                let pulse_sources = summarize_pulse_sources();
                format!(
                    "Failed to resolve Pulse source '{stripped}': no default input device available after setting PULSE_SOURCE. pulse_sources=[{pulse_sources}], input_devices=[{available_inputs}]"
                )
            });
        }

        if name.starts_with("pulse:") {
            return host.default_input_device().ok_or_else(|| {
                format!(
                    "Failed to resolve device '{name}': no default input device available. input_devices=[{available_inputs}]"
                )
            });
        }

        host.input_devices()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|d| d.id().map(|id| id.1 == name).unwrap_or(false))
            .ok_or_else(|| format!("Device '{name}' not found. input_devices=[{available_inputs}]"))
    } else {
        #[cfg(target_os = "linux")]
        {
            if let Ok(devices) = host.input_devices() {
                for dev in devices {
                    if let Ok(id) = dev.id() {
                        if id.1 == "pulse" || id.1.starts_with("default") {
                            return Ok(dev);
                        }
                    }
                }
            }
        }
        host.default_input_device().ok_or_else(|| {
            format!(
                "No input device available. input_devices=[{}]",
                available_inputs
            )
        })
    }
}
