use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::app::state::SessionState;
use crate::config::VoiceMacroCommand;
use crate::AppState;

pub async fn execute_macro_command(
    app_handle: &AppHandle,
    command: &VoiceMacroCommand,
) -> Result<(), String> {
    let steps = command.resolve_steps();
    execute_macro_steps(app_handle, &command.phrase, &steps).await
}

pub async fn execute_macro_steps(
    app_handle: &AppHandle,
    phrase_label: &str,
    steps: &[crate::config::MacroStep],
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let mut held_keys: Vec<String> = Vec::new();

    crate::log_info!(
        "[Voice Macro] Executing command '{}' ({} steps)",
        phrase_label,
        steps.len()
    );

    // Guard: Set session state to Typing for the duration of macro execution
    {
        let mut session = state.session_state.lock().unwrap();
        *session = SessionState::Typing;
    }

    let result = execute_macro_steps_inner(app_handle, steps, &mut held_keys).await;

    // Reset session state to Idle when finished
    {
        let mut session = state.session_state.lock().unwrap();
        if *session == SessionState::Typing {
            *session = SessionState::Idle;
        }
    }

    // Safety Guard: Release any keys still held down to prevent stuck keys!
    if !held_keys.is_empty() {
        crate::log_info!(
            "[Voice Macro Safety] Releasing {} held keys: {:?}",
            held_keys.len(),
            held_keys
        );
        for key in held_keys.iter().rev() {
            let _ = state.display_backend.send_key_up(app_handle, key).await;
        }
    }

    result
}

async fn execute_macro_steps_inner(
    app_handle: &AppHandle,
    steps: &[crate::config::MacroStep],
    held_keys: &mut Vec<String>,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    for (index, step) in steps.iter().enumerate() {
        if *state.session_state.lock().unwrap() != SessionState::Typing {
            crate::log_info!("[Voice Macro] Execution aborted: session cancelled");
            return Err("Voice macro execution cancelled".to_string());
        }

        match step {
            crate::config::MacroStep::KeyPress { key, hold_ms } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] KeyPress: '{}' (hold: {}ms)",
                    index + 1,
                    steps.len(),
                    key,
                    hold_ms
                );
                state
                    .display_backend
                    .send_key_combination(app_handle, key, *hold_ms)
                    .await?;
            }
            crate::config::MacroStep::KeyDown { key } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] KeyDown: '{}'",
                    index + 1,
                    steps.len(),
                    key
                );
                state.display_backend.send_key_down(app_handle, key).await?;
                if !held_keys.contains(key) {
                    held_keys.push(key.clone());
                }
            }
            crate::config::MacroStep::KeyUp { key } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] KeyUp: '{}'",
                    index + 1,
                    steps.len(),
                    key
                );
                state.display_backend.send_key_up(app_handle, key).await?;
                held_keys.retain(|k| k != key);
            }
            crate::config::MacroStep::Delay { duration_ms } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] Delay: {}ms",
                    index + 1,
                    steps.len(),
                    duration_ms
                );
                if *duration_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
                }
            }
            crate::config::MacroStep::TypeText { text } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] TypeText: '{}'",
                    index + 1,
                    steps.len(),
                    text
                );
                state
                    .display_backend
                    .type_text_hardware(app_handle, text, 0.005, 5)
                    .await?;
            }
            crate::config::MacroStep::RunCommand { command } => {
                crate::log_info!(
                    "[Voice Macro Step {}/{}] RunCommand: '{}'",
                    index + 1,
                    steps.len(),
                    command
                );
                execute_system_command(command).await?;
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub async fn execute_system_command(command_str: &str) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::process::Command;
    use tokio::time::timeout;

    crate::log_info!(
        "[Voice Macro Command] Spawning shell command: {}",
        command_str
    );

    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(command_str)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    match timeout(Duration::from_secs(15), child.wait_with_output()).await {
        Ok(Ok(output)) => {
            if output.status.success() {
                crate::log_info!("[Voice Macro Command] Command completed successfully");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let msg = if !stderr.trim().is_empty() {
                    stderr.trim()
                } else {
                    stdout.trim()
                };
                crate::log_warn!(
                    "[Voice Macro Command] Command exited with code {:?}: {}",
                    output.status.code(),
                    msg
                );
                Err(format!(
                    "Command exited with status {}: {}",
                    output.status, msg
                ))
            }
        }
        Ok(Err(e)) => Err(format!("Command execution failed: {}", e)),
        Err(_) => Err("Command timed out after 15 seconds".to_string()),
    }
}

#[cfg(target_os = "windows")]
pub async fn execute_system_command(command_str: &str) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::process::Command;
    use tokio::time::timeout;

    crate::log_info!(
        "[Voice Macro Command] Spawning Windows command: {}",
        command_str
    );

    let mut command = Command::new("cmd.exe");
    command
        .arg("/C")
        .arg(command_str)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    match timeout(Duration::from_secs(15), child.wait_with_output()).await {
        Ok(Ok(output)) => {
            if output.status.success() {
                crate::log_info!("[Voice Macro Command] Command completed successfully");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let msg = if !stderr.trim().is_empty() {
                    stderr.trim()
                } else {
                    stdout.trim()
                };
                crate::log_warn!(
                    "[Voice Macro Command] Command exited with code {:?}: {}",
                    output.status.code(),
                    msg
                );
                Err(format!(
                    "Command exited with status {}: {}",
                    output.status, msg
                ))
            }
        }
        Ok(Err(e)) => Err(format!("Command execution failed: {}", e)),
        Err(_) => Err("Command timed out after 15 seconds".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_system_command_success() {
        #[cfg(not(target_os = "windows"))]
        let cmd = "echo 'voquill test' > /dev/null";
        #[cfg(target_os = "windows")]
        let cmd = "echo voquill test > nul";

        let result = execute_system_command(cmd).await;
        assert!(result.is_ok(), "Expected success, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_execute_system_command_failure() {
        #[cfg(not(target_os = "windows"))]
        let cmd = "exit 42";
        #[cfg(target_os = "windows")]
        let cmd = "exit /b 42";

        let result = execute_system_command(cmd).await;
        assert!(result.is_err(), "Expected error on nonzero exit code");
    }
}
