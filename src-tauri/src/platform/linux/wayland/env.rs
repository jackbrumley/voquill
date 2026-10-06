/// The app's identity on Linux: Wayland app_id, GTK prgname, portal host-app
/// registration, and the launcher name (`<APP_ID>.desktop`). Must equal the
/// Tauri `identifier`. It must not end in `.desktop`: KDE's kglobalacceld
/// treats such component names as desktop-file launch shortcuts and silently
/// drops portal-registered global shortcuts for them. It must not end in
/// `.app` either, which collides with macOS bundle names.
pub const APP_ID: &str = "org.voquill.voquill";

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
            "Wayland"
        } else if type_name.contains("X11") {
            "X11"
        } else {
            "Unknown"
        };
        crate::log_info!("GDK Backend: {} ({})", backend, type_name);
    }
}

#[cfg(test)]
mod tests {
    use super::APP_ID;

    #[test]
    fn app_id_matches_tauri_identifier() {
        let tauri_config: serde_json::Value =
            serde_json::from_str(include_str!("../../../../tauri.conf.json")).unwrap();
        assert_eq!(tauri_config["identifier"], APP_ID);
    }

    #[test]
    fn packaging_files_are_named_after_app_id() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(manifest_dir
            .join(format!("packaging/{APP_ID}.desktop"))
            .is_file());
        let metainfo =
            std::fs::read_to_string(manifest_dir.join(format!("{APP_ID}.metainfo.xml"))).unwrap();
        assert!(metainfo.contains(&format!("<id>{APP_ID}</id>")));
        assert!(metainfo.contains(&format!(
            r#"<launchable type="desktop-id">{APP_ID}.desktop</launchable>"#
        )));
    }

    #[test]
    fn app_id_avoids_reserved_suffixes() {
        assert!(!APP_ID.ends_with(".desktop"));
        assert!(!APP_ID.ends_with(".app"));
    }
}
