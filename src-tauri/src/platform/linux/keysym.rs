//! The X keysym for each character of dictated text, shared by the X11
//! (XTest) and Wayland (RemoteDesktop portal) typing backends.

const XK_RETURN: u32 = 0xFF0D;
const XK_TAB: u32 = 0xFF09;
/// The legacy keysym that keyboard layouts use for '€'.
const XK_EURO_SIGN: u32 = 0x20AC;
/// Keysyms for other Unicode characters are the code point plus this offset.
const UNICODE_KEYSYM_OFFSET: u32 = 0x0100_0000;

/// Returns the keysym keyboard layouts use for `character`. Latin-1
/// characters (ASCII, umlauts, accents, ß) and '€' have legacy keysyms equal
/// to their code points; every other character uses its Unicode keysym.
pub fn keysym_for_char(character: char) -> u32 {
    let code_point = character as u32;
    match character {
        '\n' | '\r' => XK_RETURN,
        '\t' => XK_TAB,
        '€' => XK_EURO_SIGN,
        ' '..='~' | '\u{A0}'..='\u{FF}' => code_point,
        _ => UNICODE_KEYSYM_OFFSET + code_point,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_and_tabs_map_to_their_function_keys() {
        assert_eq!(keysym_for_char('\n'), XK_RETURN);
        assert_eq!(keysym_for_char('\r'), XK_RETURN);
        assert_eq!(keysym_for_char('\t'), XK_TAB);
    }

    #[test]
    fn latin1_characters_use_their_legacy_keysyms() {
        assert_eq!(keysym_for_char('a'), 0x61);
        assert_eq!(keysym_for_char('?'), 0x3F);
        assert_eq!(keysym_for_char('ü'), 0xFC); // udiaeresis
        assert_eq!(keysym_for_char('ß'), 0xDF); // ssharp
        assert_eq!(keysym_for_char('€'), XK_EURO_SIGN);
    }

    #[test]
    fn other_characters_use_unicode_keysyms() {
        assert_eq!(keysym_for_char('ł'), 0x0100_0142);
        assert_eq!(keysym_for_char('ř'), 0x0100_0159);
    }
}
