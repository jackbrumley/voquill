use once_cell::sync::Lazy;
use regex::Regex;

static MULTI_SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

const UNIVERSAL_FILLER_WORDS: &[&str] = &[
    "uh", "uhm", "umm", "uhh", "uhhh", "ehh", "ehm", "ahm", "hmm", "hm", "mmm",
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

pub fn normalize_transcription_output(text: &str) -> String {
    let collapsed = collapse_stutters(text);
    let single_spaced = MULTI_SPACE_PATTERN.replace_all(&collapsed, " ");
    single_spaced.trim().to_string()
}

pub fn clean_transcription(
    text: &str,
    filler_word_removal_enabled: bool,
    custom_filler_words: &[String],
) -> String {
    if !filler_word_removal_enabled {
        return text.to_string();
    }

    let without_fillers = remove_filler_words(text, custom_filler_words);
    let result = normalize_transcription_output(&without_fillers);

    if result != text {
        crate::log_info!(
            "Filler word removal: original=\"{}\" cleaned=\"{}\"",
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
}
