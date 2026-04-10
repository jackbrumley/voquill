use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use tauri::WebviewWindow;

pub fn apply_linux_unfocusable_hints(window: &WebviewWindow, pixels_from_bottom_logical: i32) {
    let window_clone = window.clone();
    gtk::glib::MainContext::default().invoke(move || {
        if let Ok(gtk_window) = window_clone.gtk_window() {
            if gtk_layer_shell::is_supported() {
                crate::log_info!("🛠️  Initializing Wayland Layer Shell for overlay...");

                gtk_window.init_layer_shell();
                gtk_window.set_layer(gtk_layer_shell::Layer::Overlay);
                gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
                gtk_window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::None);

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
