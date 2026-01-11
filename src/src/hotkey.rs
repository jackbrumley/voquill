use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

#[cfg(target_os = "linux")]
pub fn get_linux_key_codes(hotkey_str: &str) -> Vec<u16> {
    let binding = hotkey_str.to_lowercase();
    let parts: Vec<&str> = binding.split('+').collect();
    let mut codes = Vec::new();
    
    for part in parts {
        match part.trim() {
            "ctrl" | "control" => {
                codes.push(29); // KEY_LEFTCTRL
                codes.push(97); // KEY_RIGHTCTRL
            },
            "shift" => {
                codes.push(42); // KEY_LEFTSHIFT
                codes.push(54); // KEY_RIGHTSHIFT
            },
            "alt" => {
                codes.push(56); // KEY_LEFTALT
                codes.push(100); // KEY_RIGHTALT
            },
            "super" | "cmd" | "win" => {
                codes.push(125); // KEY_LEFTMETA
                codes.push(126); // KEY_RIGHTMETA
            },
            "space" => codes.push(57),
            "a" => codes.push(30),
            "b" => codes.push(48),
            "c" => codes.push(46),
            "d" => codes.push(32),
            "e" => codes.push(18),
            "f" => codes.push(33),
            "g" => codes.push(34),
            "h" => codes.push(35),
            "i" => codes.push(23),
            "j" => codes.push(36),
            "k" => codes.push(37),
            "l" => codes.push(38),
            "m" => codes.push(50),
            "n" => codes.push(49),
            "o" => codes.push(24),
            "p" => codes.push(25),
            "q" => codes.push(16),
            "r" => codes.push(19),
            "s" => codes.push(31),
            "t" => codes.push(20),
            "u" => codes.push(22),
            "v" => codes.push(47),
            "w" => codes.push(17),
            "x" => codes.push(45),
            "y" => codes.push(21),
            "z" => codes.push(44),
            "f1" => codes.push(59),
            "f2" => codes.push(60),
            "f3" => codes.push(61),
            "f4" => codes.push(62),
            "f5" => codes.push(63),
            "f6" => codes.push(64),
            "f7" => codes.push(65),
            "f8" => codes.push(66),
            "f9" => codes.push(67),
            "f10" => codes.push(68),
            "f11" => codes.push(87),
            "f12" => codes.push(88),
            _ => {}
        }
    }
    codes
}

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
            "" => {}, // Skip empty parts
            key => {
                key_code = Some(match key {
                    "space" => Code::Space,
                    "a" => Code::KeyA,
                    "b" => Code::KeyB,
                    "c" => Code::KeyC,
                    "d" => Code::KeyD,
                    "e" => Code::KeyE,
                    "f" => Code::KeyF,
                    "g" => Code::KeyG,
                    "h" => Code::KeyH,
                    "i" => Code::KeyI,
                    "j" => Code::KeyJ,
                    "k" => Code::KeyK,
                    "l" => Code::KeyL,
                    "m" => Code::KeyM,
                    "n" => Code::KeyN,
                    "o" => Code::KeyO,
                    "p" => Code::KeyP,
                    "q" => Code::KeyQ,
                    "r" => Code::KeyR,
                    "s" => Code::KeyS,
                    "t" => Code::KeyT,
                    "u" => Code::KeyU,
                    "v" => Code::KeyV,
                    "w" => Code::KeyW,
                    "x" => Code::KeyX,
                    "y" => Code::KeyY,
                    "z" => Code::KeyZ,
                    "f1" => Code::F1,
                    "f2" => Code::F2,
                    "f3" => Code::F3,
                    "f4" => Code::F4,
                    "f5" => Code::F5,
                    "f6" => Code::F6,
                    "f7" => Code::F7,
                    "f8" => Code::F8,
                    "f9" => Code::F9,
                    "f10" => Code::F10,
                    "f11" => Code::F11,
                    "f12" => Code::F12,
                    _ => return Err(format!("Unknown key: {}", key).into()),
                });
            }
        }
    }
    
    // If no key was specified, default to Space
    let code = key_code.unwrap_or(Code::Space);
    Ok(Shortcut::new(Some(modifiers), code))
}
