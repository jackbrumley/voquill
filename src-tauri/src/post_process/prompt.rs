pub fn build_post_process_messages(
    text: &str,
    system_prompt: &str,
    user_prompt_template: &str,
) -> Vec<serde_json::Value> {
    let user_content = user_prompt_template.replace("{transcript}", text);
    vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        }),
        serde_json::json!({
            "role": "user",
            "content": user_content
        }),
    ]
}

pub fn max_output_tokens(input_text: &str, config_tokens: u32) -> usize {
    if config_tokens > 0 {
        return config_tokens as usize;
    }
    (input_text.len()).clamp(256, 8192)
}
