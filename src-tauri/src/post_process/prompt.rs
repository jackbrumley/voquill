pub fn build_cleanup_messages(text: &str) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "role": "system",
            "content": "You are a text cleaner. Fix punctuation and capitalization. Remove filler words (um, uh, like, you know, sort of, kind of). Preserve all meaning. Output only the cleaned text, no explanation."
        }),
        serde_json::json!({
            "role": "user",
            "content": text
        }),
    ]
}
