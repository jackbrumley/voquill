use crate::app::identity::APP_ID;

pub fn configure_linux_session_environment() {
    // CRITICAL: Set app identity BEFORE any GTK/Portal operations
    // This must happen at the very start so all D-Bus calls are signed with the correct app_id
    // GTK uses the prgname as the Wayland app_id.
    gtk::glib::set_prgname(Some(APP_ID));
    gtk::glib::set_application_name("Voquill");

    // Fix for WebKitGTK crashes on Arch-based systems (like CachyOS/Manjaro)
    // This addresses the "Could not create default EGL display: EGL_BAD_PARAMETER" error.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    std::env::set_var("WEBKIT_DISABLE_GPU_SANDBOX", "1");

    if should_force_wayland_backend() {
        // Enforce Wayland backend for GTK on native Wayland sessions.
        std::env::set_var("GDK_BACKEND", "wayland");
    }
}

/// Force GTK's native Wayland backend only on a real Wayland session with a
/// reachable display, and never over a `GDK_BACKEND` the user chose
/// (APP_STANDARD.md §2).
fn should_force_wayland_backend() -> bool {
    std::env::var_os("GDK_BACKEND").is_none()
        && std::env::var("XDG_SESSION_TYPE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
        && std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty())
}

pub fn check_wayland_display() {
    // Diagnostic: Confirm we are running on Wayland
    if let Some(display) = gdk::Display::default() {
        use gtk::glib::prelude::ObjectExt;
        let type_name = display.type_().name();
        let backend = if type_name.contains("Wayland") {
            "Wayland"
        } else if type_name.contains("X11") {
            "X11"
        } else {
            "Unknown"
        };
        crate::log_info!("GDK Backend: {} ({})", backend, type_name);
    }
}
