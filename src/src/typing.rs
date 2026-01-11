use enigo::{Enigo, Settings, Keyboard};
use std::thread;
use std::time::Duration;


pub fn type_text_with_config(text: &str, typing_speed_interval: f64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_ms = (typing_speed_interval * 1000.0) as u64;
    type_text_with_speed(text, interval_ms)
}

pub fn type_text_with_speed(text: &str, interval_ms: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let settings = Settings::default();
    let mut enigo = Enigo::new(&settings)?;
    
    println!("⌨️  Preparing to type: '{}'", text);
    
    // Small delay to ensure the target application is ready
    thread::sleep(Duration::from_millis(100));
    
    // Type each character with custom delay
    for ch in text.chars() {
        if let Err(e) = enigo.text(&ch.to_string()) {
            println!("❌ Typing failed for char '{}': {}", ch, e);
            return Err(e.into());
        }
        thread::sleep(Duration::from_millis(interval_ms));
    }
    
    println!("✅ Typing complete");
    Ok(())
}
