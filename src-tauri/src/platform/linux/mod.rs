pub mod detection;
pub mod wayland;
pub mod x11;

use std::sync::Arc;
use crate::platform::traits::DisplayBackend;

pub fn initialize() -> Arc<dyn DisplayBackend> {
    match detection::detect_display_server() {
        detection::LinuxDisplayServer::Wayland => {
            Arc::new(wayland::WaylandBackend::new())
        }
        detection::LinuxDisplayServer::X11 => {
            Arc::new(x11::X11Backend::new())
        }
        detection::LinuxDisplayServer::Unknown => {
            panic!("Unsupported or unknown Linux display server");
        }
    }
}
