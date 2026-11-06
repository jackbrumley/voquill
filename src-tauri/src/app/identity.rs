//! The app's identity, per the Release and Update Standard's "App Identity"
//! section: one reverse-DNS ID used as the Tauri `identifier`, the Wayland
//! `app_id` / GTK prgname, the portal host-app registration, the tray's
//! StatusNotifierItem ID, the launcher name (`<APP_ID>.desktop`) and the
//! AppStream metainfo ID.

/// Must equal the Tauri `identifier`. It must not end in `.desktop`: KDE's
/// kglobalacceld treats such component names as desktop-file launch shortcuts
/// and silently drops portal-registered global shortcuts for them. It must not
/// end in `.app` either, which collides with macOS bundle names. Only Linux
/// uses it at run time (Windows and macOS take the identifier from
/// tauri.conf.json); the tests check it everywhere.
#[cfg(any(target_os = "linux", test))]
pub const APP_ID: &str = "org.voquill.voquill";

#[cfg(test)]
mod tests {
    use super::APP_ID;

    #[test]
    fn app_id_matches_tauri_identifier() {
        let tauri_config: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
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
