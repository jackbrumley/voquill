use crate::app::state::AppState;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use tauri::Emitter;

use super::normalization::{normalize_wayland_trigger, trigger_description_to_hotkey};

const RECORD_SHORTCUT_ID: &str = "record";

pub(crate) async fn bind_record_shortcut(
    proxy: &GlobalShortcuts<'_>,
    session: &ashpd::desktop::Session<'_, GlobalShortcuts<'_>>,
    normalized_trigger: &str,
) -> Result<String, String> {
    let shortcut = NewShortcut::new(RECORD_SHORTCUT_ID, "Dictation Hotkey")
        .preferred_trigger(Some(normalized_trigger));

    let bind_result = proxy
        .bind_shortcuts(session, &[shortcut], None)
        .await
        .map_err(|error| format!("Failed to call portal BindShortcuts: {error}"))?;

    let bound = bind_result
        .response()
        .map_err(|error| format!("Portal rejected shortcut: {error}"))?;

    crate::log_info!(
        "BindShortcuts returned {} shortcuts",
        bound.shortcuts().len()
    );

    if bound.shortcuts().is_empty() {
        return Err("OS rejected the shortcut request. The hotkey may be in use by another application or the system.".to_string());
    }

    for shortcut in bound.shortcuts() {
        crate::log_info!(
            "Wayland Global Shortcuts bound: ID='{}', Trigger='{}'",
            shortcut.id(),
            shortcut.trigger_description()
        );
        if shortcut.id() == RECORD_SHORTCUT_ID {
            return Ok(shortcut.trigger_description().to_string());
        }
    }

    Err("Portal bind succeeded but did not return the expected shortcut id 'record'.".to_string())
}

async fn has_bound_record_shortcut(
    proxy: &GlobalShortcuts<'_>,
    session: &ashpd::desktop::Session<'_, GlobalShortcuts<'_>>,
) -> Result<bool, String> {
    let listed = proxy
        .list_shortcuts(session)
        .await
        .map_err(|error| format!("Failed to call portal ListShortcuts: {error}"))?
        .response()
        .map_err(|error| format!("Failed to read shortcut list response: {error}"))?;

    Ok(listed
        .shortcuts()
        .iter()
        .any(|shortcut| shortcut.id() == RECORD_SHORTCUT_ID))
}

fn is_configure_shortcuts_unavailable(error: &ashpd::Error) -> bool {
    let message = error.to_string();
    message.contains("ConfigureShortcuts is not implemented")
        || message.contains("UnknownMethod")
        || message.contains("Method ConfigureShortcuts")
}

pub async fn try_open_linux_portal_shortcut_configuration(
    preferred_hotkey: &str,
) -> Result<bool, String> {
    let proxy = GlobalShortcuts::new()
        .await
        .map_err(|error| format!("Failed to connect to GlobalShortcuts portal: {error}"))?;

    let session = proxy
        .create_session()
        .await
        .map_err(|error| format!("Failed to create portal session: {error}"))?;

    let normalized_trigger = normalize_wayland_trigger(preferred_hotkey);
    let has_record_shortcut = has_bound_record_shortcut(&proxy, &session).await?;
    if !has_record_shortcut {
        bind_record_shortcut(&proxy, &session, &normalized_trigger).await?;
    }

    let configure_result = proxy
        .configure_shortcuts(&session, None, None::<ashpd::ActivationToken>)
        .await;
    let _ = session.close().await;

    match configure_result {
        Ok(()) => Ok(true),
        Err(ashpd::Error::RequiresVersion(_, _)) => Ok(false),
        Err(error) if is_configure_shortcuts_unavailable(&error) => {
            crate::log_warn!(
                "GlobalShortcuts ConfigureShortcuts unavailable on this desktop: {}",
                error
            );
            Ok(false)
        }
        Err(error) => Err(format!(
            "Failed to open system shortcut configuration: {error}"
        )),
    }
}

/// The portal's active trigger is authoritative on Wayland. If it diverged
/// from the configured hotkey (e.g. the user changed it in system settings),
/// rewrite the config to match and notify the UI so it shows what actually
/// triggers recording.
pub fn realign_config_to_portal_trigger(
    state: &tauri::State<'_, AppState>,
    app_handle: &tauri::AppHandle,
    active_trigger: &str,
) {
    let Some(portal_hotkey) = trigger_description_to_hotkey(active_trigger) else {
        return;
    };
    let diverged = {
        let config = state.config.lock().unwrap();
        !config.hotkey.eq_ignore_ascii_case(&portal_hotkey)
    };
    if !diverged {
        return;
    }
    crate::log_warn!(
        "Portal trigger '{}' differs from configured hotkey; realigning config to '{}'.",
        active_trigger,
        portal_hotkey
    );
    {
        let mut config = state.config.lock().unwrap();
        config.hotkey = portal_hotkey;
        let _ = crate::config::save_config(&config);
    }
    let _ = app_handle.emit("config-updated", ());
}
