use std::ffi::c_void;

use crate::platform::permissions::PlatformPermissions;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
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

pub fn request_accessibility_permission() -> Result<(), String> {
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

pub fn check_macos_permissions() -> PlatformPermissions {
    let accessibility_trusted = accessibility_is_trusted();
    crate::log_info!(
        "macOS permission status: accessibility_trusted={}",
        accessibility_trusted
    );
    PlatformPermissions {
        // CoreAudio triggers the microphone TCC prompt when the input stream opens.
        audio: true,
        shortcuts: true,
        input_emulation: accessibility_trusted,
        shortcuts_status: "ready".to_string(),
        shortcuts_detail: None,
        manual_overlay_offset_supported: true,
        overlay_positioning_detail: None,
    }
}
