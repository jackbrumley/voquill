use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const KEYBOARD_EVENT_AUTOREPEAT: i32 = 8;
const KEYBOARD_EVENT_KEYCODE: i32 = 9;
const EVENT_FLAG_SHIFT: u64 = 1 << 17;
const EVENT_FLAG_CONTROL: u64 = 1 << 18;
const EVENT_FLAG_OPTION: u64 = 1 << 19;
const EVENT_FLAG_COMMAND: u64 = 1 << 20;
const EVENT_FLAG_SECONDARY_FN: u64 = 1 << 23;
const MODIFIER_FLAGS: u64 = EVENT_FLAG_SHIFT
    | EVENT_FLAG_CONTROL
    | EVENT_FLAG_OPTION
    | EVENT_FLAG_COMMAND
    | EVENT_FLAG_SECONDARY_FN;
const EVENT_TAP_MASK: u64 = (1 << KEY_DOWN) | (1 << KEY_UP);

static FN_LISTENER_STARTED: tokio::sync::Mutex<bool> = tokio::sync::Mutex::const_new(false);
static FN_HOTKEY_DOWN: AtomicBool = AtomicBool::new(false);

type CGEventRef = *mut c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(*mut c_void, u32, CGEventRef, *mut c_void) -> CGEventRef,
        user_info: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventTapIsEnabled(tap: *mut c_void) -> bool;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: i32) -> i64;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    fn CFRunLoopRemoveSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFMachPortInvalidate(port: *mut c_void);
    fn CFRelease(value: *const c_void);
    static kCFRunLoopCommonModes: *const c_void;
}

struct FnListenerContext {
    app_handle: tauri::AppHandle,
}

pub fn hotkey_uses_fn(state: &crate::AppState) -> bool {
    state
        .config
        .lock()
        .unwrap()
        .hotkey
        .split('+')
        .any(|part| part.trim().eq_ignore_ascii_case("fn"))
}

pub async fn start_macos_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey = {
        let state = app_handle.state::<crate::AppState>();
        let hotkey = state.config.lock().unwrap().hotkey.clone();
        hotkey
    };

    if hotkey.is_empty() {
        return Err("No hotkey configured for macOS.".to_string());
    }

    let fn_shortcut = if hotkey_uses_fn(&app_handle.state::<crate::AppState>()) {
        Some(parse_fn_hotkey(&hotkey)?)
    } else {
        None
    };

    app_handle
        .global_shortcut()
        .unregister_all()
        .map_err(|error| format!("Failed to clear existing macOS hotkeys: {error}"))?;
    if fn_shortcut.is_some() {
        start_fn_hotkey_listener(app_handle.clone()).await?;
    } else {
        let shortcut = crate::hotkey::parse_hotkey_string(&hotkey)
            .map_err(|error| format!("Failed to parse hotkey string: {error}"))?;
        app_handle
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| format!("Failed to register global hotkey on macOS: {error}"))?;
    }

    crate::log_info!("macOS global hotkey registered: {}", hotkey);
    Ok(())
}

async fn start_fn_hotkey_listener(app_handle: tauri::AppHandle) -> Result<(), String> {
    if !super::permissions::input_monitoring_is_trusted() {
        super::permissions::request_input_monitoring_permission();
        return Err(
            "Fn hotkeys require Input Monitoring permission. Enable Voquill in System Settings, then register the hotkey again."
                .to_string(),
        );
    }

    let mut started = FN_LISTENER_STARTED.lock().await;
    if *started {
        return Ok(());
    }

    let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("voquill-macos-fn-hotkey".to_string())
        .spawn(move || {
            let mut context = Box::new(FnListenerContext { app_handle });
            // SAFETY: The listener owns its callback context for the lifetime of
            // the application, matching the app-wide hotkey listener lifetime.
            unsafe {
                let tap = CGEventTapCreate(
                    1,
                    0,
                    1,
                    EVENT_TAP_MASK,
                    handle_fn_hotkey_event,
                    (&mut *context as *mut FnListenerContext).cast(),
                );
                if tap.is_null() {
                    let _ = ready_sender.send(Err(
                        "Failed to create macOS Fn hotkey event tap. Check Input Monitoring permission in System Settings.".to_string(),
                    ));
                    return;
                }

                let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                if source.is_null() {
                    CFMachPortInvalidate(tap);
                    CFRelease(tap);
                    let _ = ready_sender.send(Err(
                        "Failed to create macOS Fn hotkey run loop source".to_string(),
                    ));
                    return;
                }

                let run_loop = CFRunLoopGetCurrent();
                CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                if !CGEventTapIsEnabled(tap) {
                    let _ = ready_sender.send(Err(
                        "Failed to enable macOS Fn hotkey event tap".to_string(),
                    ));
                } else if ready_sender.send(Ok(())).is_ok() {
                    crate::log_info!("macOS Fn hotkey listener started");
                    CFRunLoopRun();
                    *FN_LISTENER_STARTED.blocking_lock() = false;
                    crate::app::commands::hotkey::set_hotkey_binding_state(
                        &context.app_handle,
                        false,
                        false,
                        Some("macOS Fn hotkey listener stopped. Register the shortcut again.".to_string()),
                        None,
                    );
                    let state = context.app_handle.state::<crate::AppState>();
                    *state.hotkey_error.lock().unwrap() = Some(
                        "macOS Fn hotkey listener stopped. Register the shortcut again.".to_string(),
                    );
                }
                CFRunLoopRemoveSource(run_loop, source, kCFRunLoopCommonModes);
                CFMachPortInvalidate(tap);
                CFRelease(source);
                CFRelease(tap);
            }
        })
        .map_err(|error| format!("Failed to start macOS Fn hotkey listener: {error}"))?;

    ready_receiver
        .await
        .map_err(|error| format!("macOS Fn hotkey listener initialization failed: {error}"))??;
    *started = true;
    Ok(())
}

extern "C" fn handle_fn_hotkey_event(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    if event.is_null() || user_info.is_null() || !matches!(event_type, KEY_DOWN | KEY_UP) {
        return event;
    }

    // SAFETY: `user_info` is a boxed context held by the listener thread until
    // application exit; Core Graphics invokes this callback serially.
    let context = unsafe { &*(user_info as *const FnListenerContext) };
    let state = context.app_handle.state::<crate::AppState>();
    let configured_shortcut = {
        let config = state.config.lock().unwrap();
        parse_fn_hotkey(&config.hotkey)
    };
    let Ok(configured_shortcut) = configured_shortcut else {
        return event;
    };

    // SAFETY: The event is supplied by Core Graphics for the callback duration.
    let key_code = unsafe { CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE) as u16 };
    if key_code != configured_shortcut.key_code {
        return event;
    }

    if event_type == KEY_DOWN {
        // SAFETY: The event is supplied by Core Graphics for the callback duration.
        let is_repeat =
            unsafe { CGEventGetIntegerValueField(event, KEYBOARD_EVENT_AUTOREPEAT) != 0 };
        // SAFETY: The event is supplied by Core Graphics for the callback duration.
        let flags = unsafe { CGEventGetFlags(event) };
        if configured_shortcut.matches_flags(flags)
            && !is_repeat
            && !FN_HOTKEY_DOWN.swap(true, Ordering::SeqCst)
        {
            let app_handle = context.app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<crate::AppState>();
                crate::app::hotkey_handler::handle_hotkey_press(state, app_handle.clone()).await;
            });
        }
    } else if FN_HOTKEY_DOWN.swap(false, Ordering::SeqCst) {
        let app_handle = context.app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_handle.state::<crate::AppState>();
            crate::app::hotkey_handler::handle_hotkey_release(state).await;
        });
    }

    event
}

#[derive(Debug)]
struct FnShortcut {
    key_code: u16,
    modifiers: u64,
}

impl FnShortcut {
    fn matches_flags(&self, flags: u64) -> bool {
        flags & MODIFIER_FLAGS == self.modifiers
    }
}

fn parse_fn_hotkey(hotkey: &str) -> Result<FnShortcut, String> {
    let mut modifiers = 0;
    let mut key_code = None;
    for token in hotkey.split('+') {
        let token = token.trim().to_ascii_lowercase();
        let modifier = match token.as_str() {
            "fn" => EVENT_FLAG_SECONDARY_FN,
            "shift" => EVENT_FLAG_SHIFT,
            "ctrl" | "control" => EVENT_FLAG_CONTROL,
            "alt" | "option" => EVENT_FLAG_OPTION,
            "super" | "meta" | "command" | "cmd" => EVENT_FLAG_COMMAND,
            _ => {
                let key = super::input::parse_key(&token)?;
                if key.modifier || key_code.replace(key.code).is_some() {
                    return Err(
                        "Fn shortcuts require exactly one supported non-modifier key".to_string(),
                    );
                }
                continue;
            }
        };
        if modifiers & modifier != 0 {
            return Err(format!("Duplicate modifier '{token}' in Fn shortcut"));
        }
        modifiers |= modifier;
    }
    if modifiers & EVENT_FLAG_SECONDARY_FN == 0 {
        return Err("Fn shortcut must include the Fn modifier".to_string());
    }
    Ok(FnShortcut {
        key_code: key_code.ok_or("Fn shortcut must include a non-modifier key")?,
        modifiers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_matches_complete_fn_shortcuts() {
        let shortcut = parse_fn_hotkey("Ctrl+Fn+Z").unwrap();
        assert_eq!(shortcut.key_code, 6);
        assert!(shortcut.matches_flags(EVENT_FLAG_CONTROL | EVENT_FLAG_SECONDARY_FN));
        assert!(!shortcut.matches_flags(EVENT_FLAG_SECONDARY_FN));
        assert!(!shortcut
            .matches_flags(EVENT_FLAG_CONTROL | EVENT_FLAG_SECONDARY_FN | EVENT_FLAG_SHIFT));
        assert_eq!(parse_fn_hotkey("Fn+ArrowLeft").unwrap().key_code, 123);
        assert_eq!(parse_fn_hotkey("Fn+Digit1").unwrap().key_code, 18);
        for invalid in ["Shift+Z", "Fn", "Fn+Z+X", "Fn+Fn+Z", "Fn+Unknown", "Fn++Z"] {
            assert!(parse_fn_hotkey(invalid).is_err(), "accepted {invalid}");
        }
    }
}
