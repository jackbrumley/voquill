use crate::log_info;
use tauri::WebviewWindow;

pub fn apply_overlay_hints(window: &WebviewWindow) {
    let window_clone = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        log_info!("👻 Applying Ghost Mode attributes...");
        let _ = window_clone.set_focusable(false);
        let _ = window_clone.set_ignore_cursor_events(true);
    });
}

pub fn position_overlay_window(
    overlay_window: &WebviewWindow,
    pixels_from_bottom_logical: i32,
) -> Result<(), String> {
    use tauri::Position;
    // Standardized: Always use the OS-reported Primary Monitor for the status overlay.
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

    // Physical Pixel calculations for high-DPI accuracy
    let pixels_from_bottom_physical = (pixels_from_bottom_logical as f64 * scale_factor) as i32;
    let window_width_logical = 140.0;
    let window_height_logical = 140.0;

    let window_width_physical = (window_width_logical * scale_factor) as i32;
    let window_height_physical = (window_height_logical * scale_factor) as i32;

    let x = monitor_position.x + (monitor_size.width as i32 - window_width_physical) / 2;
    let y = monitor_position.y + monitor_size.height as i32
        - window_height_physical
        - pixels_from_bottom_physical;

    log_info!(
        "📍 Positioning overlay at Physical: {}, {} (Monitor: {:?}x{:?} at {:?}, Scale: {})",
        x,
        y,
        monitor_size.width,
        monitor_size.height,
        monitor_position,
        scale_factor
    );

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
