//! What this computer offers the speech engines: its graphics devices and
//! system memory, read through the same ggml backend registry (Vulkan on
//! Windows/Linux, MoltenVK on macOS) that runs Whisper, so what Voquill
//! reports is exactly what its engines will see, plus the processor thread
//! count from the standard library.

use serde::Serialize;
use std::ffi::CStr;
use whisper_rs::whisper_rs_sys as ggml;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum GraphicsKind {
    /// A graphics card with its own memory.
    Discrete,
    /// Graphics built into the processor, sharing system memory.
    Integrated,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphicsDevice {
    /// Human-readable name, e.g. "AMD Radeon RX 7900 XT (RADV NAVI31)".
    pub description: String,
    pub kind: GraphicsKind,
    /// Memory the device reports, in bytes.
    pub total_memory_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct HardwareProfile {
    pub graphics: Vec<GraphicsDevice>,
    /// Total system memory, as ggml's CPU device reports it (0 if unknown).
    pub system_memory_bytes: u64,
    /// Logical processor threads available to Voquill.
    pub processor_threads: usize,
}

/// Reads the hardware profile. Initializing the ggml backends can take a
/// moment, so call this off the async runtime (e.g. in `spawn_blocking`).
pub fn detect_hardware() -> HardwareProfile {
    let mut graphics = Vec::new();
    let mut system_memory_bytes = 0u64;

    // SAFETY: the ggml backend registry is initialized on first use and is
    // safe to query from any thread. Every device handle comes from the
    // registry and stays valid for the life of the process; the strings it
    // returns are owned by the registry and copied before use.
    let device_count = unsafe { ggml::ggml_backend_dev_count() };
    for index in 0..device_count {
        let device = unsafe { ggml::ggml_backend_dev_get(index) };
        if device.is_null() {
            continue;
        }
        let device_type = unsafe { ggml::ggml_backend_dev_type(device) };
        let (mut free_bytes, mut total_bytes) = (0usize, 0usize);
        unsafe { ggml::ggml_backend_dev_memory(device, &mut free_bytes, &mut total_bytes) };

        let kind = match device_type {
            ggml::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU => GraphicsKind::Discrete,
            ggml::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_IGPU => GraphicsKind::Integrated,
            ggml::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_CPU => {
                system_memory_bytes = total_bytes as u64;
                continue;
            }
            _ => continue,
        };
        let description = unsafe {
            let raw = ggml::ggml_backend_dev_description(device);
            if raw.is_null() {
                String::from("Unknown graphics device")
            } else {
                CStr::from_ptr(raw).to_string_lossy().into_owned()
            }
        };
        graphics.push(GraphicsDevice {
            description,
            kind,
            total_memory_bytes: total_bytes as u64,
        });
    }

    let profile = HardwareProfile {
        graphics,
        system_memory_bytes,
        processor_threads: std::thread::available_parallelism()
            .map(|threads| threads.get())
            .unwrap_or(1),
    };
    crate::log_info!("Hardware detected: {:?}", profile);
    profile
}
