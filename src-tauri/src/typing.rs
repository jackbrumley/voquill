use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::log_info!(
        "📋 Attempting to copy to clipboard ({} chars)...",
        text.len()
    );
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    crate::log_info!("✅ Copied to clipboard successfully");
    Ok(())
}
