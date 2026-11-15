// "Software Update" window of direct-download builds. The window's page
// (`/updater`) runs the update checks; this module only creates, shows and
// signals it.

use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};
use tracing::warn;

use crate::settings::{self, AppSettings};
use crate::{apply_settings_patch, to_command, AppState, CommandResult};

pub const WINDOW: &str = "updater";
/// Asks the updater window to check now; payload `{ manual: bool }`.
const EVENT_CHECK: &str = "updater://check";

/// The window is declared with `create: false` in tauri.conf.json so builds
/// without the updater never load it.
pub fn create_window(app: &AppHandle) {
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == WINDOW)
    else {
        warn!("No `{WINDOW}` window in tauri.conf.json");
        return;
    };
    if let Err(e) = WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
        warn!("Could not create the updater window: {e}");
    }
}

/// Shows the window and asks it to check. Manual checks ignore "Skip This
/// Version" and "Remind Me Later" and always report a result.
pub fn show(app: &AppHandle, manual: bool) {
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit_to(WINDOW, EVENT_CHECK, serde_json::json!({ "manual": manual }));
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn show_updater(app: AppHandle, manual: bool) {
    show(&app, manual);
}

/// Records the update window's choices (skip, remind later, auto-install).
/// Unlike `update_settings` this works while Clipsa is locked: the window can
/// appear at any time and these settings reveal nothing about the history.
#[tauri::command(async, rename_all = "snake_case")]
pub fn update_updater_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    settings: serde_json::Value,
) -> CommandResult<AppSettings> {
    if !settings::is_updater_patch(&settings) {
        return Err("only update preferences can be changed here".into());
    }
    to_command(apply_settings_patch(&app, &state, settings))
}
