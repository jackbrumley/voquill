use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tauri::WebviewWindow;

static OVERLAY_LAYER_SHELL_INITIALIZED: AtomicBool = AtomicBool::new(false);
static OVERLAY_LAYER_SHELL_SUPPORT: AtomicU8 = AtomicU8::new(0);

pub fn manual_overlay_offset_supported() -> bool {
    match OVERLAY_LAYER_SHELL_SUPPORT.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let supported = gtk_layer_shell::is_supported();
            OVERLAY_LAYER_SHELL_SUPPORT.store(if supported { 1 } else { 2 }, Ordering::Relaxed);
            supported
        }
    }
}

pub fn apply_linux_unfocusable_hints(window: &WebviewWindow, pixels_from_bottom_logical: i32) {
    let layer_shell_supported = gtk_layer_shell::is_supported();
    OVERLAY_LAYER_SHELL_SUPPORT.store(if layer_shell_supported { 1 } else { 2 }, Ordering::Relaxed);
    crate::log_info!(
        "🧭 Wayland overlay capability: manual_offset_supported={}",
        layer_shell_supported
    );

    let window_clone = window.clone();
    gtk::glib::MainContext::default().invoke(move || {
        if let Ok(gtk_window) = window_clone.gtk_window() {
            if layer_shell_supported {
                let was_initialized = OVERLAY_LAYER_SHELL_INITIALIZED.swap(true, Ordering::SeqCst);
                if !was_initialized {
                    crate::log_info!("🛠️  Initializing Wayland Layer Shell for overlay...");
                    gtk_window.init_layer_shell();
                    gtk_window.set_layer(gtk_layer_shell::Layer::Overlay);
                    gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
                    gtk_window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::None);
                }

                LayerShell::set_layer_shell_margin(
                    &gtk_window,
                    gtk_layer_shell::Edge::Bottom,
                    pixels_from_bottom_logical,
                );
            } else {
                crate::log_warn!(
                    "⚠️  Layer Shell not supported by compositor, using standard window hints"
                );
            }

            gtk_window.set_decorated(false);
            gtk_window.set_skip_taskbar_hint(true);
            gtk_window.set_skip_pager_hint(true);
            gtk_window.set_keep_above(true);
        }
    });
}

pub fn position_overlay_window(
    overlay_window: &WebviewWindow,
    pixels_from_bottom_logical: i32,
) -> Result<(), String> {
    if !manual_overlay_offset_supported() {
        return Ok(());
    }

    let window_clone = overlay_window.clone();

    gtk::glib::MainContext::default().invoke(move || {
        if let Ok(gtk_window) = window_clone.gtk_window() {
            if gtk_layer_shell::is_supported() {
                gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
                LayerShell::set_layer_shell_margin(
                    &gtk_window,
                    gtk_layer_shell::Edge::Bottom,
                    pixels_from_bottom_logical,
                );
            }
        }
    });

    Ok(())
}
