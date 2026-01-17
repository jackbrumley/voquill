#[cfg(target_os = "linux")]
use evdev::KeyCode;
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
            "ctrl" | "control" => KeyCode::KEY_LEFTCTRL.code(),
            "shift" => KeyCode::KEY_LEFTSHIFT.code(),
            "alt" => KeyCode::KEY_LEFTALT.code(),
            "super" | "cmd" | "win" => KeyCode::KEY_LEFTMETA.code(),
            "space" => KeyCode::KEY_SPACE.code(),
            "a" => KeyCode::KEY_A.code(), "b" => KeyCode::KEY_B.code(), "c" => KeyCode::KEY_C.code(), 
            "d" => KeyCode::KEY_D.code(), "e" => KeyCode::KEY_E.code(), "f" => KeyCode::KEY_F.code(), 
            "g" => KeyCode::KEY_G.code(), "h" => KeyCode::KEY_H.code(), "i" => KeyCode::KEY_I.code(), 
            "j" => KeyCode::KEY_J.code(), "k" => KeyCode::KEY_K.code(), "l" => KeyCode::KEY_L.code(), 
            "m" => KeyCode::KEY_M.code(), "n" => KeyCode::KEY_N.code(), "o" => KeyCode::KEY_O.code(), 
            "p" => KeyCode::KEY_P.code(), "q" => KeyCode::KEY_Q.code(), "r" => KeyCode::KEY_R.code(), 
            "s" => KeyCode::KEY_S.code(), "t" => KeyCode::KEY_T.code(), "u" => KeyCode::KEY_U.code(), 
            "v" => KeyCode::KEY_V.code(), "w" => KeyCode::KEY_W.code(), "x" => KeyCode::KEY_X.code(), 
            "y" => KeyCode::KEY_Y.code(), "z" => KeyCode::KEY_Z.code(),
            "f1" => KeyCode::KEY_F1.code(), "f2" => KeyCode::KEY_F2.code(), "f3" => KeyCode::KEY_F3.code(), 
            "f4" => KeyCode::KEY_F4.code(), "f5" => KeyCode::KEY_F5.code(), "f6" => KeyCode::KEY_F6.code(), 
            "f7" => KeyCode::KEY_F7.code(), "f8" => KeyCode::KEY_F8.code(), "f9" => KeyCode::KEY_F9.code(), 
            "f10" => KeyCode::KEY_F10.code(), "f11" => KeyCode::KEY_F11.code(), "f12" => KeyCode::KEY_F12.code(),
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

#[cfg(not(target_os = "linux"))]
pub fn parse_hardware_hotkey(_hotkey_str: &str) -> HardwareHotkey {
    HardwareHotkey::default()
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
