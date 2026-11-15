//! Lists the compute devices ggml can use, through the same backend registry
//! (Vulkan on Windows/Linux, MoltenVK on macOS) that runs Whisper on the GPU,
//! so what Voquill reports is exactly what its engines will see.

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

/// Every GPU ggml can see. Initializing the backends can take a moment, so
/// call this off the async runtime (e.g. in `spawn_blocking`).
pub fn detect_graphics_devices() -> Vec<GraphicsDevice> {
    // SAFETY: the ggml backend registry is initialized on first use and is
    // safe to query from any thread. Every device handle comes from the
    // registry and stays valid for the life of the process; the strings it
    // returns are owned by the registry and copied before use.
    let device_count = unsafe { ggml::ggml_backend_dev_count() };
    let mut devices = Vec::new();
    for index in 0..device_count {
        let device = unsafe { ggml::ggml_backend_dev_get(index) };
        if device.is_null() {
            continue;
        }
        let kind = match unsafe { ggml::ggml_backend_dev_type(device) } {
            ggml::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU => GraphicsKind::Discrete,
            ggml::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_IGPU => GraphicsKind::Integrated,
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
        let (mut free_bytes, mut total_bytes) = (0usize, 0usize);
        unsafe { ggml::ggml_backend_dev_memory(device, &mut free_bytes, &mut total_bytes) };
        devices.push(GraphicsDevice {
            description,
            kind,
            total_memory_bytes: total_bytes as u64,
        });
    }
    crate::log_info!("Graphics devices detected: {:?}", devices);
    devices
}
