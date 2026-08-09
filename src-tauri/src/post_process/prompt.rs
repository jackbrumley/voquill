pub fn build_post_process_messages(text: &str, system_prompt: &str) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": text
        }),
    ]
}
