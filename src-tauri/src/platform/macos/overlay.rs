use tauri::WebviewWindow;

pub fn apply_overlay_hints(window: &WebviewWindow) {
    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        let _ = window.set_focusable(false);
        let _ = window.set_ignore_cursor_events(true);
    });
}

pub fn position_overlay_window(
    overlay_window: &WebviewWindow,
    pixels_from_bottom_logical: i32,
) -> Result<(), String> {
    use tauri::Position;

    let monitor = overlay_window
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| {
            overlay_window
                .available_monitors()
                .ok()
                .and_then(|monitors| monitors.first().cloned())
        })
        .ok_or("No monitors found")?;
    let size = monitor.size();
    let position = monitor.position();
    let scale = monitor.scale_factor();
    let width = 260.0;
    let height = 140.0;
    let x = position.x + (size.width as i32 - (width * scale) as i32) / 2;
    let y = position.y + size.height as i32
        - (height * scale) as i32
        - (pixels_from_bottom_logical as f64 * scale) as i32;

    overlay_window
        .set_position(Position::Physical(tauri::PhysicalPosition::new(x, y)))
        .map_err(|error| error.to_string())?;
    overlay_window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| error.to_string())
}
