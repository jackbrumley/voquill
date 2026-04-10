use ashpd::desktop::global_shortcuts::GlobalShortcuts;
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub struct GlobalShortcutsCapabilities {
    pub version: u32,
    pub supports_configure_shortcuts: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortalDiagnostics {
    pub available: bool,
    pub version: u32,
    pub supports_configure_shortcuts: bool,
    pub has_record_shortcut: bool,
    pub active_trigger: Option<String>,
    pub status: String,
    pub detail: Option<String>,
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

pub async fn collect_global_shortcuts_diagnostics() -> PortalDiagnostics {
    let proxy = match GlobalShortcuts::new().await {
        Ok(proxy) => proxy,
        Err(error) => {
            return PortalDiagnostics {
                available: false,
                version: 0,
                supports_configure_shortcuts: false,
                has_record_shortcut: false,
                active_trigger: None,
                status: "unavailable".to_string(),
                detail: Some(format!("GlobalShortcuts portal unavailable: {error}")),
            };
        }
    };

    let capabilities = match detect_global_shortcuts_capabilities().await {
        Ok(capabilities) => capabilities,
        Err(error) => {
            return PortalDiagnostics {
                available: true,
                version: 0,
                supports_configure_shortcuts: false,
                has_record_shortcut: false,
                active_trigger: None,
                status: "error".to_string(),
                detail: Some(error),
            };
        }
    };

    let session = match proxy.create_session().await {
        Ok(session) => session,
        Err(error) => {
            return PortalDiagnostics {
                available: true,
                version: capabilities.version,
                supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
                has_record_shortcut: false,
                active_trigger: None,
                status: "error".to_string(),
                detail: Some(format!("Failed to create GlobalShortcuts session: {error}")),
            };
        }
    };

    let list_response = proxy.list_shortcuts(&session).await;
    let diagnostics = match list_response {
        Ok(request) => match request.response() {
            Ok(listed) => {
                let active_trigger = listed
                    .shortcuts()
                    .iter()
                    .find(|shortcut| shortcut.id() == "record")
                    .map(|shortcut| shortcut.trigger_description().to_string());

                PortalDiagnostics {
                    available: true,
                    version: capabilities.version,
                    supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
                    has_record_shortcut: active_trigger.is_some(),
                    active_trigger,
                    status: if capabilities.version >= 1 {
                        "ready".to_string()
                    } else {
                        "unsupported".to_string()
                    },
                    detail: None,
                }
            }
            Err(error) => PortalDiagnostics {
                available: true,
                version: capabilities.version,
                supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
                has_record_shortcut: false,
                active_trigger: None,
                status: "error".to_string(),
                detail: Some(format!("Failed to parse ListShortcuts response: {error}")),
            },
        },
        Err(error) => PortalDiagnostics {
            available: true,
            version: capabilities.version,
            supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
            has_record_shortcut: false,
            active_trigger: None,
            status: "error".to_string(),
            detail: Some(format!("Failed to call ListShortcuts: {error}")),
        },
    };

    let _ = session.close().await;
    diagnostics
}
