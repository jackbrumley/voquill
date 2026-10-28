#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod permissions;
pub mod traits;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::initialize;

#[cfg(target_os = "macos")]
pub use macos::initialize;

#[cfg(target_os = "windows")]
pub use windows::initialize;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn initialize() -> std::sync::Arc<dyn traits::DisplayBackend> {
    unimplemented!("Platform initialization not implemented for this OS yet");
}
