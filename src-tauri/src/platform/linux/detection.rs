#[derive(Debug)]
pub enum LinuxDisplayServer {
    Wayland,
    X11,
    Unknown,
}

pub fn is_wayland_session() -> bool {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }

    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        if session_type.eq_ignore_ascii_case("wayland") {
            return true;
        }
    }

    if let Ok(gdk_backend) = std::env::var("GDK_BACKEND") {
        if gdk_backend.to_lowercase().contains("wayland") {
            return true;
        }
    }

    false
}

pub fn is_x11_session() -> bool {
    if std::env::var("DISPLAY").is_ok() {
        return true;
    }

    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        if session_type.eq_ignore_ascii_case("x11") {
            return true;
        }
    }

    false
}

pub fn detect_display_server() -> LinuxDisplayServer {
    let is_wayland = is_wayland_session();
    let is_x11 = is_x11_session();

    if is_wayland {
        LinuxDisplayServer::Wayland
    } else if is_x11 {
        LinuxDisplayServer::X11
    } else {
        LinuxDisplayServer::Unknown
    }
}
