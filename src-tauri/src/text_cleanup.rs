use once_cell::sync::Lazy;
use regex::Regex;

static MULTI_SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

const UNIVERSAL_FILLER_WORDS: &[&str] = &[
    "um", "umm", "ummm", "uh", "uhm", "uhh", "uhhh", "ah", "ahh", "ahhh", "ahm", "eh", "ehh",
    "ehm", "er", "err", "errr", "hmm", "hm", "mmm", "mm",
];

fn collapse_stutters(text: &str) -> String {
    let mut result = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let is_alphabetic = word.chars().any(|c| c.is_alphabetic());

        if !is_alphabetic {
            result.push(word);
            i += 1;
            continue;
        }

        let lower = word.to_lowercase();
        let mut count = 1;
        while i + count < words.len() {
            if words[i + count].to_lowercase() == lower {
                count += 1;
            } else {
                break;
            }
        }

        if count >= 3 {
            result.push(word);
        } else {
            for j in 0..count {
                result.push(words[i + j]);
            }
        }

        i += count;
    }

    result.join(" ")
}

pub fn remove_filler_words(text: &str, custom_filler_words: &[String]) -> String {
    let mut result = text.to_string();

    for word in UNIVERSAL_FILLER_WORDS {
        let pattern = Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word)))
            .expect("Invalid filler word pattern");
        result = pattern.replace_all(&result, "").to_string();
    }

    for word in custom_filler_words {
        if word.is_empty() {
            continue;
        }
        let pattern = Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word)))
            .expect("Invalid custom filler word pattern");
        result = pattern.replace_all(&result, "").to_string();
    }

    result
}

pub fn has_alphanumeric_content(text: &str) -> bool {
    text.chars().any(|c| c.is_alphanumeric())
}

fn sanitize_leading_punctuation(text: &str) -> String {
    let trimmed = text.trim_start();
    let mut chars = trimmed.chars().peekable();
    let mut stripped_any = false;

    while let Some(&c) = chars.peek() {
        if matches!(c, ',' | ';' | ':' | '.' | '-' | '–' | '—' | '…' | '!' | '?') {
            stripped_any = true;
            chars.next();
        } else if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let remaining: String = chars.collect();
    let remaining_trimmed = remaining.trim_start();

    if stripped_any && !remaining_trimmed.is_empty() {
        let mut result = String::with_capacity(remaining_trimmed.len());
        let mut capitalized = false;
        for ch in remaining_trimmed.chars() {
            if !capitalized && ch.is_alphabetic() {
                for upper in ch.to_uppercase() {
                    result.push(upper);
                }
                capitalized = true;
            } else {
                result.push(ch);
            }
        }
        result
    } else {
        remaining_trimmed.to_string()
    }
}

pub fn normalize_transcription_output(text: &str) -> String {
    let trimmed = text.trim();
    if !has_alphanumeric_content(trimmed) {
        return String::new();
    }
    let collapsed = collapse_stutters(trimmed);
    let single_spaced = MULTI_SPACE_PATTERN.replace_all(&collapsed, " ");
    let sanitized = sanitize_leading_punctuation(&single_spaced);
    sanitized.trim().to_string()
}

pub fn clean_transcription(
    text: &str,
    filler_word_removal_enabled: bool,
    custom_filler_words: &[String],
) -> String {
    let candidate = if filler_word_removal_enabled {
        remove_filler_words(text, custom_filler_words)
    } else {
        text.to_string()
    };

    let result = normalize_transcription_output(&candidate);

    if result != text {
        crate::log_info!(
            "Transcription cleaned: original=\"{}\" cleaned=\"{}\"",
            text,
            result
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_universal_fillers() {
        let result = remove_filler_words("uh hello world umm testing", &[]);
        assert_eq!(result, " hello world  testing");
    }

    #[test]
    fn test_remove_ums_and_ahs_and_ers() {
        let input = "um I think that ah this is er definitely working";
        let cleaned = clean_transcription(input, true, &[]);
        assert_eq!(cleaned, "I think that this is definitely working");
    }

    #[test]
    fn test_remove_fillers_with_punctuation() {
        let result = remove_filler_words("uh, hello umm. world", &[]);
        assert_eq!(result, " hello  world");
    }

    #[test]
    fn test_custom_fillers_supplement_builtin() {
        let custom = vec!["literally".to_string(), "actually".to_string()];
        let result = remove_filler_words("uh literally I was umm actually there", &custom);
        let cleaned = normalize_transcription_output(&result);
        assert_eq!(cleaned, "I was there");
    }

    #[test]
    fn test_no_fillers_unchanged() {
        let result = remove_filler_words("hello world this is fine", &[]);
        assert_eq!(result, "hello world this is fine");
    }

    #[test]
    fn test_collapse_stutters_three_or_more() {
        assert_eq!(collapse_stutters("I I I think so"), "I think so");
    }

    #[test]
    fn test_collapse_stutters_many_repeats() {
        assert_eq!(
            collapse_stutters("why why why why would you"),
            "why would you"
        );
    }

    #[test]
    fn test_collapse_stutters_two_preserved() {
        assert_eq!(collapse_stutters("no no is fine"), "no no is fine");
    }

    #[test]
    fn test_collapse_stutters_mixed_case() {
        assert_eq!(collapse_stutters("No NO no no no"), "No");
    }

    #[test]
    fn test_clean_transcription_disabled() {
        let result = clean_transcription("uh hello world umm testing", false, &[]);
        assert_eq!(result, "uh hello world umm testing");
    }

    #[test]
    fn test_clean_transcription_enabled() {
        let result = clean_transcription("uh hello world umm testing", true, &[]);
        assert_eq!(result, "hello world testing");
    }

    #[test]
    fn test_clean_with_stutter_and_fillers() {
        let result = clean_transcription("uh I I I really think umm this is great", true, &[]);
        assert_eq!(result, "I really think this is great");
    }

    #[test]
    fn test_clean_with_custom_fillers() {
        let custom = vec!["basically".to_string()];
        let result = clean_transcription("uh basically I was umm there", true, &custom);
        assert_eq!(result, "I was there");
    }

    #[test]
    fn test_strip_leading_comma_with_uppercase() {
        let result = normalize_transcription_output(
            ", I agree with almost all of those defaults. The only one I'd probably disagree with is the pixels from the bottom.",
        );
        assert_eq!(
            result,
            "I agree with almost all of those defaults. The only one I'd probably disagree with is the pixels from the bottom."
        );
    }

    #[test]
    fn test_strip_leading_comma_with_lowercase_auto_capitalizes() {
        let result = normalize_transcription_output(
            ", the difference between front-end and back-end defaults, that shouldn't even exist.",
        );
        assert_eq!(
            result,
            "The difference between front-end and back-end defaults, that shouldn't even exist."
        );
    }

    #[test]
    fn test_strip_leading_dash_and_ellipsis() {
        assert_eq!(
            normalize_transcription_output("- this should be cleaned"),
            "This should be cleaned"
        );
        assert_eq!(
            normalize_transcription_output("... wait a minute"),
            "Wait a minute"
        );
        assert_eq!(
            normalize_transcription_output("; next item on the list"),
            "Next item on the list"
        );
        assert_eq!(
            normalize_transcription_output(": here is another one"),
            "Here is another one"
        );
    }

    #[test]
    fn test_pure_punctuation_returns_empty_string() {
        assert_eq!(normalize_transcription_output("."), "");
        assert_eq!(normalize_transcription_output(","), "");
        assert_eq!(normalize_transcription_output("..."), "");
        assert_eq!(normalize_transcription_output("-"), "");
        assert_eq!(normalize_transcription_output("—"), "");
        assert_eq!(normalize_transcription_output("!?"), "");
        assert_eq!(normalize_transcription_output(" . , ; - "), "");
        assert_eq!(clean_transcription(".", true, &[]), "");
        assert_eq!(clean_transcription(".", false, &[]), "");
    }

    #[test]
    fn test_internal_punctuation_preserved() {
        let input = "Hello, world! This is a test: 123 items.";
        assert_eq!(normalize_transcription_output(input), input);
    }
}
