use crate::config::VoiceMacroCommand;

#[derive(Clone, Debug, serde::Serialize)]
pub struct MacroMatchResult {
    pub matched: bool,
    pub similarity: f32,
    pub transcript: String,
    pub matched_command: Option<VoiceMacroCommand>,
}

pub fn normalize_phrase(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_alphanumeric() || ch.is_whitespace() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(' ');
        }
    }
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut prev_row: Vec<usize> = (0..=len_b).collect();
    let mut curr_row: Vec<usize> = vec![0; len_b + 1];

    for (i, &char_a) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, &char_b) in b_chars.iter().enumerate() {
            let cost = if char_a == char_b { 0 } else { 1 };
            curr_row[j + 1] = std::cmp::min(
                std::cmp::min(prev_row[j + 1] + 1, curr_row[j] + 1),
                prev_row[j] + cost,
            );
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[len_b]
}

pub fn string_similarity(a: &str, b: &str) -> f32 {
    let a_clean = normalize_phrase(a);
    let b_clean = normalize_phrase(b);

    if a_clean == b_clean {
        return 1.0;
    }

    let a_collapsed: String = a_clean.chars().filter(|c| !c.is_whitespace()).collect();
    let b_collapsed: String = b_clean.chars().filter(|c| !c.is_whitespace()).collect();

    if a_collapsed == b_collapsed {
        return 1.0;
    }

    let max_len = a_collapsed.len().max(b_collapsed.len());
    if max_len == 0 {
        return 1.0;
    }

    let dist = levenshtein_distance(&a_collapsed, &b_collapsed);
    (1.0 - (dist as f32 / max_len as f32)).max(0.0)
}

pub fn find_best_match(
    transcript: &str,
    trigger_word: &str,
    commands: &[VoiceMacroCommand],
) -> MacroMatchResult {
    let clean_transcript = normalize_phrase(transcript);
    let clean_trigger = normalize_phrase(trigger_word);

    let candidate_phrase = if !clean_trigger.is_empty() {
        if !clean_transcript.contains(&clean_trigger) {
            return MacroMatchResult {
                matched: false,
                similarity: 0.0,
                transcript: clean_transcript,
                matched_command: None,
            };
        }
        clean_transcript.replacen(&clean_trigger, "", 1)
    } else {
        clean_transcript.clone()
    };

    let clean_candidate = candidate_phrase.trim();
    if clean_candidate.is_empty() {
        return MacroMatchResult {
            matched: false,
            similarity: 0.0,
            transcript: clean_transcript,
            matched_command: None,
        };
    }

    let mut best_cmd: Option<VoiceMacroCommand> = None;
    let mut best_sim = 0.0f32;

    for command in commands {
        for phrase in command.all_phrases() {
            let clean_macro = normalize_phrase(&phrase);
            if clean_macro.is_empty() {
                continue;
            }

            // 1. Direct similarity (including space-insensitive collapse)
            let sim = string_similarity(clean_candidate, &clean_macro);
            if sim > best_sim {
                best_sim = sim;
                best_cmd = Some(command.clone());
            }

            // 2. Windowed similarity across words if candidate is longer than macro
            let candidate_words: Vec<&str> = clean_candidate.split_whitespace().collect();
            let macro_words: Vec<&str> = clean_macro.split_whitespace().collect();
            let m_len = macro_words.len();

            if candidate_words.len() >= m_len && m_len > 0 {
                let min_w = m_len.saturating_sub(1).max(1);
                let max_w = (m_len + 1).min(candidate_words.len());

                for w_len in min_w..=max_w {
                    for window in candidate_words.windows(w_len) {
                        let window_str = window.join(" ");
                        let w_sim = string_similarity(&window_str, &clean_macro);
                        if w_sim > best_sim {
                            best_sim = w_sim;
                            best_cmd = Some(command.clone());
                        }
                    }
                }
            }
        }
    }

    // Dynamic confidence threshold:
    // Single word phrases need higher confidence (>= 0.85) to avoid false positives.
    // Multi-word phrases can trigger at >= 0.75 because accidental multi-word collisions are rare.
    let required_threshold = if clean_candidate.split_whitespace().count() <= 1 {
        0.85
    } else {
        0.75
    };

    let matched = best_sim >= required_threshold;

    MacroMatchResult {
        matched,
        similarity: best_sim,
        transcript: clean_transcript,
        matched_command: if matched { best_cmd } else { None },
    }
}

pub fn match_phrase(
    transcript: &str,
    trigger_word: &str,
    commands: &[VoiceMacroCommand],
) -> Option<VoiceMacroCommand> {
    let res = find_best_match(transcript, trigger_word, commands);
    if res.matched {
        res.matched_command
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cmd(
        id: &str,
        phrase: &str,
        phrases: Vec<&str>,
        steps: Vec<crate::config::MacroStep>,
    ) -> VoiceMacroCommand {
        VoiceMacroCommand {
            id: id.to_string(),
            phrase: phrase.to_string(),
            phrases: phrases.into_iter().map(String::from).collect(),
            steps,
            key_combination: None,
            hold_ms: None,
            delay_after_ms: None,
            sound_mode: crate::config::MacroSoundMode::Default,
            sound_tts_text: None,
            sound_tts_voice: None,
            sound_tts_speed: None,
            sound_tts_effect: None,
            sound_tts_pitch: None,
        }
    }

    #[test]
    fn test_normalize_phrase() {
        assert_eq!(normalize_phrase("Airstrike, please!"), "airstrike please");
        assert_eq!(normalize_phrase("  Drop   Smoke ... "), "drop smoke");
    }

    #[test]
    fn test_match_phrase_direct() {
        let commands = vec![
            test_cmd(
                "1",
                "airstrike",
                vec![],
                vec![crate::config::MacroStep::KeyPress {
                    key: "F3".into(),
                    hold_ms: 50,
                }],
            ),
            test_cmd(
                "2",
                "drop smoke",
                vec![],
                vec![crate::config::MacroStep::KeyPress {
                    key: "Ctrl+2".into(),
                    hold_ms: 50,
                }],
            ),
        ];

        let matched = match_phrase("airstrike", "", &commands);
        assert!(matched.is_some());
        let steps = matched.unwrap().resolve_steps();
        assert_eq!(
            steps,
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50
            }]
        );

        let matched_smoke = match_phrase("let's drop smoke now", "", &commands);
        assert!(matched_smoke.is_some());
        let steps_smoke = matched_smoke.unwrap().resolve_steps();
        assert_eq!(
            steps_smoke,
            vec![crate::config::MacroStep::KeyPress {
                key: "Ctrl+2".into(),
                hold_ms: 50
            }]
        );

        let no_match = match_phrase("call artillery", "", &commands);
        assert!(no_match.is_none());
    }

    #[test]
    fn test_match_phrase_with_trigger_word() {
        let commands = vec![test_cmd(
            "1",
            "airstrike",
            vec![],
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
        )];

        assert!(match_phrase("airstrike", "computer", &commands).is_none());

        let matched = match_phrase("computer airstrike", "computer", &commands);
        assert!(matched.is_some());
        let steps = matched.unwrap().resolve_steps();
        assert_eq!(
            steps,
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50
            }]
        );

        let matched_punct = match_phrase("Computer, airstrike!", "computer", &commands);
        assert!(matched_punct.is_some());
    }

    #[test]
    fn test_match_phrase_space_insensitive_and_compound_words() {
        let commands = vec![test_cmd(
            "1",
            "call airstrike",
            vec![],
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
        )];

        let matched = match_phrase("call air strike", "", &commands);
        assert!(matched.is_some());

        let matched_collapsed = match_phrase("callairstrike", "", &commands);
        assert!(matched_collapsed.is_some());
    }

    #[test]
    fn test_match_phrase_phonetic_fuzzy_tolerance() {
        let commands = vec![test_cmd(
            "1",
            "call airstrike",
            vec![],
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
        )];

        let res = find_best_match("Coal Air Strike.", "", &commands);
        assert!(res.matched);
        assert!(res.similarity >= 0.78);
        assert_eq!(res.matched_command.unwrap().phrase, "call airstrike");
    }

    #[test]
    fn test_match_phrase_conversational_subsequence() {
        let commands = vec![test_cmd(
            "1",
            "call airstrike",
            vec![],
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
        )];

        let res = find_best_match("Can we call an airstrike on that hill?", "", &commands);
        assert!(res.matched);
        assert_eq!(res.matched_command.unwrap().phrase, "call airstrike");
    }

    #[test]
    fn test_match_phrase_multiple_aliases() {
        let commands = vec![test_cmd(
            "1",
            "call airstrike",
            vec!["airstrike", "rain fire", "strike"],
            vec![crate::config::MacroStep::KeyPress {
                key: "F3".into(),
                hold_ms: 50,
            }],
        )];

        assert!(match_phrase("call airstrike", "", &commands).is_some());
        assert!(match_phrase("rain fire", "", &commands).is_some());
        assert!(match_phrase("strike", "", &commands).is_some());

        let res = find_best_match("Please rain fire on my mark", "", &commands);
        assert!(res.matched);
        assert_eq!(res.matched_command.unwrap().id, "1");
    }
}
