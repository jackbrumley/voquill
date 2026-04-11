use tauri::WebviewWindow;

pub fn apply_linux_unfocusable_hints(_window: &WebviewWindow, _pixels_from_bottom_logical: i32) {
    crate::log_info!("🛠️  Initializing X11 overlay hints...");
    // Future X11 specific unfocusable window hints using winit or direct X11 can be placed here.
    // For now, the ghost mode setup in main.rs covers most use cases.
}

pub fn position_overlay_window(
    overlay_window: &WebviewWindow,
    pixels_from_bottom_logical: i32,
) -> Result<(), String> {
    use tauri::Position;

    // Standardized fallback logic similar to Windows
    let monitor = overlay_window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| {
            overlay_window
                .available_monitors()
                .ok()
                .and_then(|m| m.first().cloned())
        })
        .ok_or("No monitors found")?;

    let monitor_size = monitor.size();
    let monitor_position = monitor.position();
    let scale_factor = monitor.scale_factor();

    let pixels_from_bottom_physical = (pixels_from_bottom_logical as f64 * scale_factor) as i32;
    let window_width_logical = 260.0;
    let window_height_logical = 140.0;

    let window_width_physical = (window_width_logical * scale_factor) as i32;
    let window_height_physical = (window_height_logical * scale_factor) as i32;

    let x = monitor_position.x + (monitor_size.width as i32 - window_width_physical) / 2;
    let y = monitor_position.y + monitor_size.height as i32
        - window_height_physical
        - pixels_from_bottom_physical;

    overlay_window
        .set_position(Position::Physical(tauri::PhysicalPosition::new(x, y)))
        .map_err(|e| e.to_string())?;
    overlay_window
        .set_size(tauri::LogicalSize::new(
            window_width_logical,
            window_height_logical,
        ))
        .map_err(|e| e.to_string())?;

    Ok(())
}
