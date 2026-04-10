pub fn enforce_wayland() {
    // CRITICAL: Set app identity BEFORE any GTK/Portal operations
    // This must happen at the very start so all D-Bus calls are signed with the correct app_id
    std::env::set_var("WAYLAND_APP_ID", "org.voquill.foss");
    gtk::glib::set_prgname(Some("org.voquill.foss"));
    gtk::glib::set_application_name("Voquill");

    // Fix for WebKitGTK crashes on Arch-based systems (like CachyOS/Manjaro)
    // This addresses the "Could not create default EGL display: EGL_BAD_PARAMETER" error.
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    std::env::set_var("WEBKIT_DISABLE_GPU_SANDBOX", "1");

    // Pre-flight check: Ensure we are in a Wayland session
    // Voquill strictly requires Wayland for Layer Shell positioning and security protocols.
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

    if !is_wayland {
        let is_x11 = std::env::var("DISPLAY").is_ok();
        if is_x11 {
            eprintln!("\n\x1b[1;31m[Voquill Error] Wayland Session Required\x1b[0m");
            eprintln!("Voquill is built strictly for Wayland to ensure proper window positioning (via Layer Shell) and secure hardware access.");
            eprintln!("Your current session appears to be X11/XWayland, which is not supported.");
            eprintln!("Please log into a native Wayland session (GNOME, KDE, or Hyprland) to use this application.\n");
        } else {
            eprintln!("\n\x1b[1;31m[Voquill Error] No Wayland Display Detected\x1b[0m");
            eprintln!("Voquill requires a Wayland session to run. If you are in a Wayland session, ensure WAYLAND_DISPLAY is set.\n");
        }
        std::process::exit(1);
    }

    // Strictly Wayland: Enforce the Wayland backend for GTK.
    // This prevents fallbacks to XWayland/X11 which break Layer Shell positioning.
    std::env::set_var("GDK_BACKEND", "wayland");
}

pub fn check_wayland_display() {
    // Diagnostic: Confirm we are running on Wayland
    if let Some(display) = gdk::Display::default() {
        use gtk::glib::prelude::ObjectExt;
        let type_name = display.type_().name();
        let backend = if type_name.contains("Wayland") {
            "Wayland ✅"
        } else {
            "X11/Unknown ❌ (Positioning will likely fail)"
        };
        crate::log_info!("🖥️  GDK Backend: {} ({})", backend, type_name);
    }
}
