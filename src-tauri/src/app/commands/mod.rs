pub mod config;
pub mod diagnostics;
pub mod file_import;
pub mod hotkey;
pub mod platform;
pub mod post_process;
pub mod recording;
pub mod transcription;
pub mod update;
pub mod voice_macro;
pub mod window;

pub use config::*;
pub use diagnostics::*;
pub use file_import::*;
pub use hotkey::*;
pub use platform::*;
pub use post_process::*;
pub use recording::*;
pub use transcription::*;
pub use update::*;
pub use voice_macro::*;
pub use window::*;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    const MAIN_RS: &str = include_str!("../../main.rs");
    const APP_PERMISSIONS: &str = include_str!("../../../permissions/app.toml");

    /// The commands registered with `tauri::generate_handler!` in `main.rs`.
    fn registered_commands() -> BTreeSet<&'static str> {
        let handler = MAIN_RS
            .split("generate_handler![")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("main.rs registers commands with generate_handler!");
        handler
            .split(',')
            .map(str::trim)
            .filter(|command| !command.is_empty())
            .collect()
    }

    /// The commands the frontend may call, from `permissions/app.toml`.
    fn allowed_commands() -> BTreeSet<&'static str> {
        let allow_list = APP_PERMISSIONS
            .split("commands.allow = [")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("app.toml has a commands.allow list");
        allow_list
            .split(',')
            .map(|entry| entry.trim().trim_matches('"'))
            .filter(|command| !command.is_empty())
            .collect()
    }

    /// Tauri rejects any command missing from the allow-list, so a command
    /// registered without an entry silently fails in the UI.
    #[test]
    fn every_registered_command_is_allowed_and_nothing_else() {
        let registered = registered_commands();
        let allowed = allowed_commands();
        let not_allowed: Vec<_> = registered.difference(&allowed).collect();
        let not_registered: Vec<_> = allowed.difference(&registered).collect();
        assert!(
            not_allowed.is_empty() && not_registered.is_empty(),
            "permissions/app.toml is out of sync with main.rs: \
             missing from the allow-list {not_allowed:?}, \
             allowed but not registered {not_registered:?}"
        );
    }
}
