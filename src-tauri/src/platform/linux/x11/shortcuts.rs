use crate::AppState;
use tauri::{Manager, Emitter};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ModMask, GrabMode};
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::Event;
use std::sync::atomic::{AtomicBool, Ordering};

static ENGINE_RUNNING: AtomicBool = AtomicBool::new(false);

pub async fn start_x11_hotkey_engine(app_handle: tauri::AppHandle) {
    if ENGINE_RUNNING.load(Ordering::SeqCst) {
        crate::log_info!("🔄 X11 Hotkey engine already running.");
        return;
    }

    crate::log_info!("🚀 X11 Global Shortcuts Engine starting...");

    let hotkey_str = {
        let state = app_handle.state::<AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    if hotkey_str.is_empty() {
        crate::log_info!("⚠️ No hotkey configured for X11.");
        return;
    }

    // Connect to X11 server
    let (conn, screen_num) = match RustConnection::connect(None) {
        Ok(c) => c,
        Err(e) => {
            crate::log_info!("❌ Failed to connect to X11: {:?}", e);
            return;
        }
    };

    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Convert string hotkey to X11 Modifiers and KeyCode (Simplified implementation)
    // For a production app, we would use a robust keymap parser here.
    // For now, we stub this out and grab an example key if parsing fails.
    
    // Example: Super + Space
    let modifiers = ModMask::M4; // Super/Meta
    let keycode: u8 = 65; // Example space keycode on many systems

    // Ungrab any previous just in case
    let _ = conn.ungrab_key(x11rb::NONE as u8, root, ModMask::ANY);

    match conn.grab_key(
        true, // owner_events
        root,
        modifiers.into(),
        keycode,
        GrabMode::ASYNC,
        GrabMode::ASYNC,
    ) {
        Ok(_) => crate::log_info!("✅ Grabbed key {} with modifiers {:?}", keycode, modifiers),
        Err(e) => {
            crate::log_info!("❌ Failed to grab key: {:?}", e);
            return;
        }
    };

    let _ = conn.flush();
    ENGINE_RUNNING.store(true, Ordering::SeqCst);

    tauri::async_runtime::spawn_blocking(move || {
        crate::log_info!("👂 Listening for X11 events...");
        loop {
            if !ENGINE_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            match conn.wait_for_event() {
                Ok(Event::KeyPress(event)) => {
                    crate::log_info!("🎤 X11: Hotkey Pressed (keycode: {})", event.detail);
                    let h_handle = app_handle.clone();
                    tauri::async_runtime::block_on(async move {
                        let state = h_handle.state::<AppState>();
                        let _ = crate::start_recording(state, h_handle.clone()).await;
                        if let Some(w) = h_handle.get_webview_window("main") {
                            let _ = w.emit("hotkey-pressed", ());
                        }
                    });
                }
                Ok(Event::KeyRelease(event)) => {
                    crate::log_info!("⏹️  X11: Hotkey Released (keycode: {})", event.detail);
                    let h_handle = app_handle.clone();
                    tauri::async_runtime::block_on(async move {
                        let state = h_handle.state::<AppState>();
                        let _ = crate::stop_recording(state).await;
                        if let Some(w) = h_handle.get_webview_window("main") {
                            let _ = w.emit("hotkey-released", ());
                        }
                    });
                }
                Ok(_) => {
                    // Ignore other events
                }
                Err(e) => {
                    crate::log_info!("❌ X11 Event Error: {:?}", e);
                    break;
                }
            }
        }
        crate::log_info!("✅ X11 Hotkey engine stopped cleanly.");
    });
}

