use arboard::Clipboard;

/// Normalize typographic/Unicode characters to their ASCII equivalents so the
/// text can be produced reliably by hardware typing engines on any platform.
/// This is the single source of truth for character mapping — all platform
/// backends receive pre-normalized text.
pub fn normalize_for_typing(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\u{2013}' | '\u{2014}' | '\u{2015}' => output.push('-'),
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => output.push('\''),
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => output.push('"'),
            '\u{2022}' => output.push('-'),
            '\u{2026}' => output.push_str("..."),
            '\u{00A0}' | '\u{2009}' | '\u{200A}' | '\u{202F}' => output.push(' '),
            _ => output.push(character),
        }
    }
    output
}

pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::log_info!("Attempting to copy to clipboard ({} chars)...", text.len());
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    crate::log_info!("Copied to clipboard successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_em_dash() {
        assert_eq!(normalize_for_typing("hello—world"), "hello-world");
    }

    #[test]
    fn test_normalize_en_dash() {
        assert_eq!(normalize_for_typing("hello–world"), "hello-world");
    }

    #[test]
    fn test_normalize_curly_quotes() {
        assert_eq!(normalize_for_typing("\u{201C}hello\u{201D}"), "\"hello\"");
        assert_eq!(normalize_for_typing("\u{2018}hello\u{2019}"), "'hello'");
    }

    #[test]
    fn test_normalize_ellipsis() {
        assert_eq!(normalize_for_typing("hello..."), "hello...");
        assert_eq!(normalize_for_typing("hello\u{2026}"), "hello...");
    }

    #[test]
    fn test_normalize_non_breaking_space() {
        assert_eq!(normalize_for_typing("hello\u{00A0}world"), "hello world");
    }

    #[test]
    fn test_normalize_ascii_preserved() {
        assert_eq!(
            normalize_for_typing("Hello, world! 123."),
            "Hello, world! 123."
        );
    }
}
