use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

#[derive(Clone, Debug, Default)]
pub struct HardwareHotkey {
    pub modifiers: Vec<u16>, // e.g., 29 (LeftCtrl), 42 (LeftShift)
    pub key: u16,           // e.g., 57 (Space)
    pub all_codes: Vec<u16>, // Every code that must be down
}

#[cfg(target_os = "linux")]
pub fn parse_hardware_hotkey(hotkey_str: &str) -> HardwareHotkey {
    let binding = hotkey_str.to_lowercase();
    let parts: Vec<&str> = binding.split('+').collect();
    let mut modifiers = Vec::new();
    let mut key = 0;
    let mut all_codes = Vec::new();
    
    for part in parts {
        let part = part.trim();
        let code = match part {
            "ctrl" | "control" => 29, // Left Ctrl
            "shift" => 42,           // Left Shift
            "alt" => 56,             // Left Alt
            "super" | "cmd" | "win" => 125, // Left Meta
            "space" => 57,
            "a" => 30, "b" => 48, "c" => 46, "d" => 32, "e" => 18,
            "f" => 33, "g" => 34, "h" => 35, "i" => 23, "j" => 36,
            "k" => 37, "l" => 38, "m" => 50, "n" => 49, "o" => 24,
            "p" => 25, "q" => 16, "r" => 19, "s" => 31, "t" => 20,
            "u" => 22, "v" => 47, "w" => 17, "x" => 45, "y" => 21,
            "z" => 44,
            "f1" => 59, "f2" => 60, "f3" => 61, "f4" => 62, "f5" => 63,
            "f6" => 64, "f7" => 65, "f8" => 66, "f9" => 67, "f10" => 68,
            "f11" => 87, "f12" => 88,
            _ => 0
        };

        if code == 0 { continue; }

        if ["ctrl", "control", "shift", "alt", "super", "cmd", "win"].contains(&part) {
            modifiers.push(code);
        } else {
            key = code;
        }
        all_codes.push(code);
    }

    HardwareHotkey {
        modifiers,
        key,
        all_codes,
    }
}

// Keep the standard parser for other platforms
#[allow(dead_code)]
pub fn parse_hotkey_string(hotkey_str: &str) -> Result<Shortcut, Box<dyn std::error::Error + Send + Sync>> {
    let binding = hotkey_str.to_lowercase();
    let parts: Vec<&str> = binding.split('+').collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code = None;
    
    for part in parts {
        match part.trim() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "cmd" | "win" => modifiers |= Modifiers::SUPER,
            "" => {},
            key => {
                key_code = Some(match key {
                    "space" => Code::Space,
                    "a" => Code::KeyA, "b" => Code::KeyB, "c" => Code::KeyC, "d" => Code::KeyD, "e" => Code::KeyE,
                    "f" => Code::KeyF, "g" => Code::KeyG, "h" => Code::KeyH, "i" => Code::KeyI, "j" => Code::KeyJ,
                    "k" => Code::KeyK, "l" => Code::KeyL, "m" => Code::KeyM, "n" => Code::KeyN, "o" => Code::KeyO,
                    "p" => Code::KeyP, "q" => Code::KeyQ, "r" => Code::KeyR, "s" => Code::KeyS, "t" => Code::KeyT,
                    "u" => Code::KeyU, "v" => Code::KeyV, "w" => Code::KeyW, "x" => Code::KeyX, "y" => Code::KeyY,
                    "z" => Code::KeyZ,
                    "f1" => Code::F1, "f2" => Code::F2, "f3" => Code::F3, "f4" => Code::F4, "f5" => Code::F5,
                    "f6" => Code::F6, "f7" => Code::F7, "f8" => Code::F8, "f9" => Code::F9, "f10" => Code::F10,
                    "f11" => Code::F11, "f12" => Code::F12,
                    _ => return Err(format!("Unknown key: {}", key).into()),
                });
            }
        }
    }
    
    let code = key_code.unwrap_or(Code::Space);
    Ok(Shortcut::new(Some(modifiers), code))
}
