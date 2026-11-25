//! Windows global hotkeys.
//!
//! Normal combos use `tauri-plugin-global-shortcut` (`RegisterHotKey`). OS-reserved
//! two-key chords like Alt+Space and Win+Space never reach that API, so those use a
//! `WH_KEYBOARD_LL` hook that can see and swallow them before the shell handles them.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLKHF_UP, MSG,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Combos Windows reserves that RegisterHotKey cannot reliably own.
pub const RESERVED_HOTKEY_PRESETS: &[&str] = &["alt+space", "super+space"];

static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static SUPER_DOWN: AtomicBool = AtomicBool::new(false);
static HOTKEY_HELD: AtomicBool = AtomicBool::new(false);
static CAPTURE_RESERVED: AtomicBool = AtomicBool::new(false);
static CAPTURE_EMITTED: AtomicBool = AtomicBool::new(false);
static DICTATION_VK: AtomicU32 = AtomicU32::new(0);
static DICTATION_CTRL: AtomicBool = AtomicBool::new(false);
static DICTATION_SHIFT: AtomicBool = AtomicBool::new(false);
static DICTATION_ALT: AtomicBool = AtomicBool::new(false);
static DICTATION_SUPER: AtomicBool = AtomicBool::new(false);
static DICTATION_ACTIVE: AtomicBool = AtomicBool::new(false);

static APP_HANDLE: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);
static HOOK_THREAD: Mutex<Option<HookThread>> = Mutex::new(None);

struct HookThread {
    thread_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HookShortcut {
    ctrl: bool,
    shift: bool,
    alt: bool,
    super_key: bool,
    vk: u32,
}

impl HookShortcut {
    fn matches_modifiers(&self) -> bool {
        CTRL_DOWN.load(Ordering::SeqCst) == self.ctrl
            && SHIFT_DOWN.load(Ordering::SeqCst) == self.shift
            && ALT_DOWN.load(Ordering::SeqCst) == self.alt
            && SUPER_DOWN.load(Ordering::SeqCst) == self.super_key
    }

    fn as_hotkey_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.shift {
            parts.push("shift");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.super_key {
            parts.push("super");
        }
        parts.push(match self.vk {
            code if code == u32::from(VK_SPACE.0) => "space",
            _ => "key",
        });
        parts.join("+")
    }
}

/// Whether this binding must use the low-level hook instead of RegisterHotKey.
pub fn needs_low_level_hook(hotkey: &str) -> bool {
    parse_reserved_hotkey(hotkey).is_some()
}

fn parse_reserved_hotkey(hotkey: &str) -> Option<HookShortcut> {
    let normalized = hotkey.trim().to_ascii_lowercase();
    let parts: Vec<&str> = normalized
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();

    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut super_key = false;
    let mut primary: Option<u32> = None;

    for part in parts {
        match part {
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            "alt" | "option" => alt = true,
            "super" | "meta" | "win" | "cmd" | "command" => super_key = true,
            "space" => {
                if primary.replace(u32::from(VK_SPACE.0)).is_some() {
                    return None;
                }
            }
            _ => return None,
        }
    }

    let vk = primary?;
    // Only the OS-reserved two-key chords go through the hook.
    let is_alt_space = alt && !ctrl && !shift && !super_key && vk == u32::from(VK_SPACE.0);
    let is_win_space = super_key && !ctrl && !shift && !alt && vk == u32::from(VK_SPACE.0);
    if is_alt_space || is_win_space {
        Some(HookShortcut {
            ctrl,
            shift,
            alt,
            super_key,
            vk,
        })
    } else {
        None
    }
}

fn reserved_capture_targets() -> [HookShortcut; 2] {
    [
        HookShortcut {
            ctrl: false,
            shift: false,
            alt: true,
            super_key: false,
            vk: u32::from(VK_SPACE.0),
        },
        HookShortcut {
            ctrl: false,
            shift: false,
            alt: false,
            super_key: true,
            vk: u32::from(VK_SPACE.0),
        },
    ]
}

pub async fn start_windows_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey_string = {
        let state = app_handle.state::<crate::AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    crate::log_info!("Re-registering hotkey: {}", hotkey_string);
    let _ = app_handle.global_shortcut().unregister_all();

    {
        let mut slot = APP_HANDLE.lock().unwrap();
        *slot = Some(app_handle.clone());
    }

    if let Some(shortcut) = parse_reserved_hotkey(&hotkey_string) {
        set_dictation_target(Some(shortcut));
        ensure_hook_thread()?;
        crate::log_info!(
            "Windows reserved hotkey hooked via WH_KEYBOARD_LL: {}",
            hotkey_string
        );
        return Ok(());
    }

    set_dictation_target(None);
    maybe_stop_hook_thread();

    let shortcut = crate::hotkey::parse_hotkey_string(&hotkey_string)
        .map_err(|error| format!("Failed to parse hotkey string: {error}"))?;
    app_handle
        .global_shortcut()
        .register(shortcut)
        .map_err(|error| format!("Failed to register global hotkey: {error}"))?;
    crate::log_info!("Global hotkey registered: {}", hotkey_string);
    Ok(())
}

/// Keeps the LL hook alive while the in-app capture modal is open so reserved
/// chords can be detected even though the WebView never receives them.
pub fn set_windows_hotkey_capture_mode(
    app_handle: &tauri::AppHandle,
    is_configuring: bool,
) -> Result<(), String> {
    {
        let mut slot = APP_HANDLE.lock().unwrap();
        *slot = Some(app_handle.clone());
    }
    CAPTURE_RESERVED.store(is_configuring, Ordering::SeqCst);
    CAPTURE_EMITTED.store(false, Ordering::SeqCst);
    if is_configuring {
        ensure_hook_thread()?;
        crate::log_info!("Windows reserved-hotkey capture mode enabled");
    } else {
        maybe_stop_hook_thread();
        crate::log_info!("Windows reserved-hotkey capture mode disabled");
    }
    Ok(())
}

fn set_dictation_target(shortcut: Option<HookShortcut>) {
    if let Some(shortcut) = shortcut {
        DICTATION_ACTIVE.store(true, Ordering::SeqCst);
        DICTATION_VK.store(shortcut.vk, Ordering::SeqCst);
        DICTATION_CTRL.store(shortcut.ctrl, Ordering::SeqCst);
        DICTATION_SHIFT.store(shortcut.shift, Ordering::SeqCst);
        DICTATION_ALT.store(shortcut.alt, Ordering::SeqCst);
        DICTATION_SUPER.store(shortcut.super_key, Ordering::SeqCst);
    } else {
        DICTATION_ACTIVE.store(false, Ordering::SeqCst);
        DICTATION_VK.store(0, Ordering::SeqCst);
        HOTKEY_HELD.store(false, Ordering::SeqCst);
    }
}

fn current_dictation_target() -> Option<HookShortcut> {
    if !DICTATION_ACTIVE.load(Ordering::SeqCst) {
        return None;
    }
    Some(HookShortcut {
        ctrl: DICTATION_CTRL.load(Ordering::SeqCst),
        shift: DICTATION_SHIFT.load(Ordering::SeqCst),
        alt: DICTATION_ALT.load(Ordering::SeqCst),
        super_key: DICTATION_SUPER.load(Ordering::SeqCst),
        vk: DICTATION_VK.load(Ordering::SeqCst),
    })
}

fn ensure_hook_thread() -> Result<(), String> {
    let mut slot = HOOK_THREAD.lock().unwrap();
    if slot.is_some() {
        return Ok(());
    }

    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("voquill-windows-hotkey-hook".to_string())
        .spawn(move || {
            // SAFETY: Installing a process-wide WH_KEYBOARD_LL hook; the proc is
            // static and the message loop below keeps the hook thread alive.
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0)
            };
            let hook = match hook {
                Ok(hook) => hook,
                Err(error) => {
                    let _ = ready_tx.send(Err(format!(
                        "Failed to install Windows keyboard hook: {error}"
                    )));
                    return;
                }
            };

            let thread_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
            if ready_tx.send(Ok(thread_id)).is_err() {
                unsafe {
                    let _ = UnhookWindowsHookEx(hook);
                }
                return;
            }

            crate::log_info!("Windows WH_KEYBOARD_LL hotkey hook started");
            let mut message = MSG::default();
            while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            unsafe {
                let _ = UnhookWindowsHookEx(hook);
            }
            HOTKEY_HELD.store(false, Ordering::SeqCst);
            crate::log_info!("Windows WH_KEYBOARD_LL hotkey hook stopped");
        })
        .map_err(|error| format!("Failed to start Windows hotkey hook thread: {error}"))?;

    let thread_id = ready_rx
        .recv()
        .map_err(|error| format!("Windows hotkey hook thread failed to start: {error}"))??;
    *slot = Some(HookThread { thread_id });
    Ok(())
}

fn maybe_stop_hook_thread() {
    if DICTATION_ACTIVE.load(Ordering::SeqCst) || CAPTURE_RESERVED.load(Ordering::SeqCst) {
        return;
    }

    let Some(thread) = HOOK_THREAD.lock().unwrap().take() else {
        return;
    };
    unsafe {
        let _ = PostThreadMessageW(thread.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
    }
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    if (info.flags & LLKHF_INJECTED).0 != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let vk = info.vkCode;
    let key_up =
        matches!(wparam.0 as u32, WM_KEYUP | WM_SYSKEYUP) || (info.flags & LLKHF_UP).0 != 0;
    let key_down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN) && !key_up;

    update_modifier_state(vk, key_up);

    if key_down {
        if CAPTURE_RESERVED.load(Ordering::SeqCst) {
            for target in reserved_capture_targets() {
                if vk == target.vk && target.matches_modifiers() {
                    if !CAPTURE_EMITTED.swap(true, Ordering::SeqCst) {
                        emit_captured_hotkey(target.as_hotkey_string());
                    }
                    return LRESULT(1);
                }
            }
        }

        if let Some(target) = current_dictation_target() {
            if vk == target.vk && target.matches_modifiers() {
                if !HOTKEY_HELD.swap(true, Ordering::SeqCst) {
                    dispatch_press();
                }
                return LRESULT(1);
            }
        }
    } else if key_up {
        if CAPTURE_RESERVED.load(Ordering::SeqCst) && vk == u32::from(VK_SPACE.0) {
            CAPTURE_EMITTED.store(false, Ordering::SeqCst);
        }

        if let Some(target) = current_dictation_target() {
            let primary_released = vk == target.vk;
            let required_modifier_released = is_modifier_vk(vk) && !target.matches_modifiers();
            if HOTKEY_HELD.load(Ordering::SeqCst) && (primary_released || required_modifier_released)
            {
                if HOTKEY_HELD.swap(false, Ordering::SeqCst) {
                    dispatch_release();
                }
                if primary_released {
                    return LRESULT(1);
                }
            }
        }
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn is_modifier_vk(vk: u32) -> bool {
    matches!(
        vk,
        code if code == u32::from(VK_CONTROL.0)
            || code == u32::from(VK_LCONTROL.0)
            || code == u32::from(VK_RCONTROL.0)
            || code == u32::from(VK_SHIFT.0)
            || code == u32::from(VK_LSHIFT.0)
            || code == u32::from(VK_RSHIFT.0)
            || code == u32::from(VK_MENU.0)
            || code == u32::from(VK_LMENU.0)
            || code == u32::from(VK_RMENU.0)
            || code == u32::from(VK_LWIN.0)
            || code == u32::from(VK_RWIN.0)
    )
}

fn update_modifier_state(vk: u32, is_up: bool) {
    let down = !is_up;
    match vk {
        code if code == u32::from(VK_CONTROL.0)
            || code == u32::from(VK_LCONTROL.0)
            || code == u32::from(VK_RCONTROL.0) =>
        {
            CTRL_DOWN.store(down, Ordering::SeqCst);
        }
        code if code == u32::from(VK_SHIFT.0)
            || code == u32::from(VK_LSHIFT.0)
            || code == u32::from(VK_RSHIFT.0) =>
        {
            SHIFT_DOWN.store(down, Ordering::SeqCst);
        }
        code if code == u32::from(VK_MENU.0)
            || code == u32::from(VK_LMENU.0)
            || code == u32::from(VK_RMENU.0) =>
        {
            ALT_DOWN.store(down, Ordering::SeqCst);
        }
        code if code == u32::from(VK_LWIN.0) || code == u32::from(VK_RWIN.0) => {
            SUPER_DOWN.store(down, Ordering::SeqCst);
        }
        _ => {}
    }
}

fn emit_captured_hotkey(hotkey: String) {
    let app_handle = APP_HANDLE.lock().unwrap().clone();
    let Some(app_handle) = app_handle else {
        return;
    };
    let _ = app_handle.emit("windows-reserved-hotkey-captured", hotkey);
}

fn dispatch_press() {
    let app_handle = APP_HANDLE.lock().unwrap().clone();
    let Some(app_handle) = app_handle else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<crate::AppState>();
        crate::app::hotkey_handler::handle_hotkey_press(state, app_handle.clone()).await;
    });
}

fn dispatch_release() {
    let app_handle = APP_HANDLE.lock().unwrap().clone();
    let Some(app_handle) = app_handle else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<crate::AppState>();
        crate::app::hotkey_handler::handle_hotkey_release(state).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_alt_space_and_win_space() {
        assert!(needs_low_level_hook("alt+space"));
        assert!(needs_low_level_hook("Alt+Space"));
        assert!(needs_low_level_hook("super+space"));
        assert!(needs_low_level_hook("win+space"));
        assert!(!needs_low_level_hook("ctrl+shift+space"));
        assert!(!needs_low_level_hook("alt+shift+space"));
        assert!(!needs_low_level_hook("ctrl+space"));
    }
}
