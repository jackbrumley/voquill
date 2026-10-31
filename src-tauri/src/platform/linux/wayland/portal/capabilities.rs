use ashpd::desktop::global_shortcuts::GlobalShortcuts;

#[derive(Clone, Copy, Debug)]
pub struct GlobalShortcutsCapabilities {
    pub version: u32,
    pub supports_configure_shortcuts: bool,
}

pub async fn detect_global_shortcuts_capabilities() -> Result<GlobalShortcutsCapabilities, String> {
    let proxy = GlobalShortcuts::new()
        .await
        .map_err(|error| format!("Failed to connect to GlobalShortcuts portal: {error}"))?;

    use std::ops::Deref;
    let version = proxy
        .deref()
        .get_property::<u32>("version")
        .await
        .map_err(|error| format!("Failed to read GlobalShortcuts portal version: {error}"))?;

    Ok(GlobalShortcutsCapabilities {
        version,
        supports_configure_shortcuts: version >= 2,
    })
}
