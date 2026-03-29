pub mod linux;
pub mod permissions;
pub mod traits;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::initialize as initialize;

#[cfg(target_os = "windows")]
pub use windows::initialize as initialize;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn initialize() -> std::sync::Arc<dyn traits::DisplayBackend> {
    unimplemented!("Platform initialization not implemented for this OS yet");
}
