#[derive(Debug)]
pub enum LinuxDisplayServer {
    Wayland,
    X11,
    Unknown,
}

pub fn detect_display_server() -> LinuxDisplayServer {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let is_x11 = std::env::var("DISPLAY").is_ok();

    if is_wayland {
        LinuxDisplayServer::Wayland
    } else if is_x11 {
        LinuxDisplayServer::X11
    } else {
        LinuxDisplayServer::Unknown
    }
}
