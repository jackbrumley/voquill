//! What a bug report needs about this machine (App Standard sections 3-4):
//! written as the session log header and shown in Settings -> Debug.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub app_version: String,
    pub build_profile: String,
    pub executable: String,
    pub os: String,
    pub distribution: String,
    pub architecture: String,
    pub desktop: String,
    pub session_type: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            // A dev build and an installed package can share a version.
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
            executable: std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|error| format!("unknown ({error})")),
            os: std::env::consts::OS.to_string(),
            distribution: distribution_name().unwrap_or_else(|| "unknown".to_string()),
            architecture: std::env::consts::ARCH.to_string(),
            desktop: env_or_unknown("XDG_CURRENT_DESKTOP"),
            session_type: session_type(),
        }
    }

    /// One `key=value` line per field, for the copy button.
    pub fn summary(&self) -> String {
        format!(
            "app=Voquill {}\nbuild={}\nexecutable={}\nos={}\ndistribution={}\narchitecture={}\ndesktop={}\nsession_type={}",
            self.app_version,
            self.build_profile,
            self.executable,
            self.os,
            self.distribution,
            self.architecture,
            self.desktop,
            self.session_type
        )
    }

    /// The same fields on one line, for the session log header.
    pub fn header_line(&self) -> String {
        self.summary().replace('\n', " | ")
    }
}

/// The distro's human-readable name (`PRETTY_NAME` in `/etc/os-release`).
pub fn distribution_name() -> Option<String> {
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;
    contents.lines().find_map(|line| {
        line.strip_prefix("PRETTY_NAME=")
            .map(|value| value.trim_matches('"').to_string())
    })
}

fn env_or_unknown(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Wayland or X11, from the session's environment.
fn session_type() -> String {
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        if !session_type.trim().is_empty() {
            return session_type;
        }
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "wayland".to_string()
    } else if std::env::var_os("DISPLAY").is_some() {
        "x11".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::SystemInfo;

    #[test]
    fn summary_names_every_field() {
        let summary = SystemInfo::collect().summary();
        for key in [
            "app=Voquill ",
            "build=",
            "executable=",
            "os=",
            "distribution=",
            "architecture=",
            "desktop=",
            "session_type=",
        ] {
            assert!(summary.contains(key), "missing {key} in {summary}");
        }
    }

    #[test]
    fn header_line_is_a_single_line() {
        assert!(!SystemInfo::collect().header_line().contains('\n'));
    }
}
