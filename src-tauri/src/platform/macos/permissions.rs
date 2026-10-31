use std::ffi::c_void;

use crate::platform::permissions::PlatformPermissions;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    static kAXTrustedCheckOptionPrompt: *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: *const c_void;
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        num_values: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *const c_void;
    fn CFRelease(value: *const c_void);
}

pub fn accessibility_is_trusted() -> bool {
    // SAFETY: This is a side-effect-free macOS accessibility trust query.
    unsafe { AXIsProcessTrusted() }
}

pub fn input_monitoring_is_trusted() -> bool {
    // SAFETY: This is a side-effect-free macOS Input Monitoring trust query.
    unsafe { CGPreflightListenEventAccess() }
}

/// Fn hotkeys are read through an event tap, which needs Input Monitoring.
/// Prompts when it is missing and reports what the user must do.
pub fn ensure_input_monitoring_permission() -> Result<(), String> {
    if input_monitoring_is_trusted() {
        return Ok(());
    }
    // SAFETY: macOS owns the permission prompt and returns immediately.
    let granted = unsafe { CGRequestListenEventAccess() };
    crate::log_info!(
        "macOS Input Monitoring permission requested: granted={}",
        granted
    );
    if granted {
        return Ok(());
    }
    Err(
        "Fn hotkeys require Input Monitoring permission. Enable Voquill in System Settings, then return to the app."
            .to_string(),
    )
}

/// Everything the configured hotkey and typing need: Accessibility always
/// (typing), Input Monitoring only for Fn hotkeys.
pub fn request_input_permissions(hotkey: &str) -> Result<(), String> {
    request_accessibility_permission()?;
    if super::shortcuts::hotkey_uses_fn(hotkey) {
        ensure_input_monitoring_permission()?;
    }
    Ok(())
}

fn request_accessibility_permission() -> Result<(), String> {
    let trusted = accessibility_is_trusted();
    crate::log_info!(
        "macOS Accessibility permission requested: trusted={}",
        trusted
    );
    if trusted {
        return Ok(());
    }

    let keys = [unsafe { kAXTrustedCheckOptionPrompt }];
    let values = [unsafe { kCFBooleanTrue }];
    // SAFETY: CoreFoundation retains the static key and value for the duration
    // of this call; the temporary dictionary is released immediately after.
    unsafe {
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            std::ptr::null(),
            std::ptr::null(),
        );
        if options.is_null() {
            return Err("Failed to create macOS Accessibility permission request.".to_string());
        }
        let _ = AXIsProcessTrustedWithOptions(options);
        CFRelease(options);
    }

    Err(
        "macOS is requesting Accessibility permission for this Voquill.app. Enable it in System Settings, then return to the app."
            .to_string(),
    )
}

pub fn check_macos_permissions(config: &crate::config::Config) -> PlatformPermissions {
    let accessibility_trusted = accessibility_is_trusted();
    let fn_hotkey = super::shortcuts::hotkey_uses_fn(&config.hotkey);
    let input_monitoring_trusted = input_monitoring_is_trusted();
    let input_monitoring_missing = fn_hotkey && !input_monitoring_trusted;
    crate::log_info!(
        "macOS permission status: accessibility_trusted={}, input_monitoring_trusted={}",
        accessibility_trusted,
        input_monitoring_trusted
    );
    PlatformPermissions {
        // CoreAudio triggers the microphone TCC prompt when the input stream opens.
        audio: true,
        shortcuts: !input_monitoring_missing,
        input_emulation: accessibility_trusted,
        input_emulation_restoring: false,
        shortcuts_status: if input_monitoring_missing {
            "input_monitoring_required".to_string()
        } else {
            "ready".to_string()
        },
        shortcuts_detail: input_monitoring_missing.then(|| {
            "Fn hotkeys require Input Monitoring permission in System Settings.".to_string()
        }),
        manual_overlay_offset_supported: true,
        overlay_positioning_detail: None,
    }
}
