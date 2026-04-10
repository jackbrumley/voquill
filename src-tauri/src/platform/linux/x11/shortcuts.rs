use crate::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, GrabMode, ModMask};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

static ENGINE_RUNNING: AtomicBool = AtomicBool::new(false);

pub async fn start_x11_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    if ENGINE_RUNNING.load(Ordering::SeqCst) {
        crate::log_info!("🔄 X11 Hotkey engine already running.");
        return Ok(());
    }

    crate::log_info!("🚀 X11 Global Shortcuts Engine starting...");

    let hotkey_str = {
        let state = app_handle.state::<AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    if hotkey_str.is_empty() {
        return Err("No hotkey configured for X11.".to_string());
    }

    let (conn, screen_num) = RustConnection::connect(None)
        .map_err(|error| format!("Failed to connect to X11: {error:?}"))?;

    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let modifiers = ModMask::M4;
    let keycode: u8 = 65;

    let _ = conn.ungrab_key(x11rb::NONE as u8, root, ModMask::ANY);

    conn.grab_key(
        true,
        root,
        modifiers.into(),
        keycode,
        GrabMode::ASYNC,
        GrabMode::ASYNC,
    )
    .map_err(|error| format!("Failed to grab X11 key: {error:?}"))?;

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
                Ok(_) => {}
                Err(error) => {
                    crate::log_info!("❌ X11 Event Error: {:?}", error);
                    break;
                }
            }
        }
        crate::log_info!("✅ X11 Hotkey engine stopped cleanly.");
    });

    Ok(())
}
