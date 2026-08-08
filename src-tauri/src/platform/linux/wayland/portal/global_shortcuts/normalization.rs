pub fn normalize_wayland_trigger(hotkey: &str) -> String {
    let mut parts: Vec<String> = hotkey
        .split('+')
        .map(|segment| segment.trim().to_string())
        .collect();
    if parts.is_empty() {
        return hotkey.to_string();
    }
    if parts.len() < 2 {
        return normalize_wayland_key_name(&parts[0]);
    }

    let mut key = parts.pop().unwrap_or_default();
    key = normalize_wayland_key_name(&key);

    let mut modifiers = parts;
    for modifier in &mut modifiers {
        *modifier = modifier.to_uppercase();
    }

    modifiers.join("+") + "+" + &key
}

fn normalize_wayland_key_name(key: &str) -> String {
    if key.eq_ignore_ascii_case("space") {
        "space".to_string()
    } else if key.eq_ignore_ascii_case("enter") || key.eq_ignore_ascii_case("return") {
        "Return".to_string()
    } else if key.len() >= 2
        && (key.starts_with('f') || key.starts_with('F'))
        && key[1..].chars().all(|character| character.is_ascii_digit())
    {
        format!("F{}", &key[1..])
    } else {
        key.to_string()
    }
}

pub fn trigger_description_matches_request(description: &str, normalized_request: &str) -> bool {
    let mut modifiers: Vec<&str> = Vec::new();
    if description.contains("<Control>") {
        modifiers.push("CTRL");
    }
    if description.contains("<Shift>") {
        modifiers.push("SHIFT");
    }
    if description.contains("<Alt>") {
        modifiers.push("ALT");
    }
    if description.contains("<Super>") {
        modifiers.push("SUPER");
    }
    modifiers.sort_unstable();

    let key = description
        .split('>')
        .next_back()
        .map(str::trim)
        .unwrap_or_default();

    let description_normalized = if modifiers.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", modifiers.join("+"), key)
    };

    description_normalized.eq_ignore_ascii_case(normalized_request)
}

pub fn trigger_description_to_hotkey(description: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if description.contains("<Control>") {
        parts.push("ctrl".to_string());
    }
    if description.contains("<Shift>") {
        parts.push("shift".to_string());
    }
    if description.contains("<Alt>") {
        parts.push("alt".to_string());
    }
    if description.contains("<Super>") {
        parts.push("super".to_string());
    }

    let key = description
        .split('>')
        .next_back()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())?
        .to_lowercase();

    parts.push(key);
    Some(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::normalize_wayland_trigger;

    #[test]
    fn normalize_space_shortcut() {
        assert_eq!(
            normalize_wayland_trigger("ctrl+shift+space"),
            "CTRL+SHIFT+space"
        );
    }

    #[test]
    fn normalize_enter_shortcut() {
        assert_eq!(normalize_wayland_trigger("ctrl+enter"), "CTRL+Return");
    }

    #[test]
    fn normalize_function_shortcut() {
        assert_eq!(normalize_wayland_trigger("f8"), "F8");
        assert_eq!(normalize_wayland_trigger("ctrl+f8"), "CTRL+F8");
    }
}
