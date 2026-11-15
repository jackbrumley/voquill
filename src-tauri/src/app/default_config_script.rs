//! Hands the backend's default settings to every webview before its scripts
//! run, so the frontend's pre-load placeholder config comes from
//! `Config::default()` instead of a hand-written copy that could drift.

use crate::config::Config;

/// The global the frontend reads in `src/defaultConfig.ts`.
const GLOBAL_NAME: &str = "__VOQUILL_DEFAULT_CONFIG__";

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    let defaults =
        serde_json::to_string(&Config::default()).expect("the default config always serializes");
    tauri::plugin::Builder::new("voquill-default-config")
        .js_init_script(format!("window.{GLOBAL_NAME} = Object.freeze({defaults});"))
        .build()
}
