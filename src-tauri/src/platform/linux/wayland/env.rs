pub fn configure_linux_session_environment() {
    // CRITICAL: Set app identity BEFORE any GTK/Portal operations
    // This must happen at the very start so all D-Bus calls are signed with the correct app_id
    std::env::set_var("WAYLAND_APP_ID", "org.voquill.foss");
    gtk::glib::set_prgname(Some("org.voquill.foss"));
    gtk::glib::set_application_name("Voquill");

    // Fix for WebKitGTK crashes on Arch-based systems (like CachyOS/Manjaro)
    // This addresses the "Could not create default EGL display: EGL_BAD_PARAMETER" error.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    std::env::set_var("WEBKIT_DISABLE_GPU_SANDBOX", "1");

    if crate::platform::linux::detection::is_wayland_session() {
        // Enforce Wayland backend for GTK on native Wayland sessions.
        std::env::set_var("GDK_BACKEND", "wayland");
    }
}

pub fn check_wayland_display() {
    // Diagnostic: Confirm we are running on Wayland
    if let Some(display) = gdk::Display::default() {
        use gtk::glib::prelude::ObjectExt;
        let type_name = display.type_().name();
        let backend = if type_name.contains("Wayland") {
            "Wayland ✅"
        } else if type_name.contains("X11") {
            "X11 ✅"
        } else {
            "Unknown ⚠️"
        };
        crate::log_info!("🖥️  GDK Backend: {} ({})", backend, type_name);
    }
}
