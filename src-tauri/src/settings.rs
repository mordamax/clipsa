// User-facing application settings.

use serde::{Deserialize, Serialize};

use crate::error::{ClipsaError, ClipsaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub max_items: u32,
    pub auto_cleanup_days: u32,
    pub enable_monitoring: bool,
    pub ignore_sensitive: bool,
    pub excluded_apps: Vec<String>,
    pub capture_images: bool,
    pub theme: Theme,
    pub compact_mode: bool,
    pub show_timestamps: bool,
    pub items_per_page: u32,
    pub global_hotkey: String,
    pub launch_at_login: bool,
    /// Off until the user opts in (first-run screen or settings); it is the
    /// app's only network access.
    pub check_for_updates: bool,
    /// Download and install updates found by the automatic check without
    /// asking; they take effect on the next launch.
    pub auto_install_updates: bool,
    /// Version the user chose "Skip This Version" for; the automatic check
    /// ignores exactly this version.
    pub skipped_update_version: String,
    /// Unix time in milliseconds until which "Remind Me Later" silences the
    /// automatic check. 0 = not snoozed.
    pub update_snoozed_until: u64,
    /// Whether the first-run screen has been completed.
    pub onboarding_completed: bool,
    /// Default paste/copy mode strips formatting.
    pub paste_plain_text: bool,
    /// Require OS authentication to view history after inactivity.
    pub app_lock: bool,
    /// 0 = lock as soon as every Clipsa window is hidden.
    pub lock_after_minutes: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_items: 1000,
            auto_cleanup_days: 30,
            enable_monitoring: true,
            ignore_sensitive: true,
            excluded_apps: Vec::new(),
            capture_images: true,
            theme: Theme::System,
            compact_mode: false,
            show_timestamps: true,
            items_per_page: 50,
            global_hotkey: "CmdOrCtrl+Shift+V".into(),
            launch_at_login: false,
            check_for_updates: false,
            auto_install_updates: false,
            skipped_update_version: String::new(),
            update_snoozed_until: 0,
            paste_plain_text: false,
            app_lock: false,
            lock_after_minutes: 5,
            onboarding_completed: false,
        }
    }
}

impl AppSettings {
    /// Apply a partial JSON object on top of `self`, normalise and validate.
    /// Unknown keys and wrongly typed values are rejected.
    pub fn merged(&self, patch: serde_json::Value) -> ClipsaResult<Self> {
        let serde_json::Value::Object(patch) = patch else {
            return Err(ClipsaError::InvalidSetting(
                "settings must be an object".into(),
            ));
        };
        let mut current = serde_json::to_value(self)?;
        let fields = current
            .as_object_mut()
            .expect("AppSettings serializes to an object");
        for (key, value) in patch {
            if !fields.contains_key(&key) {
                return Err(ClipsaError::InvalidSetting(format!(
                    "unknown setting `{key}`"
                )));
            }
            fields.insert(key, value);
        }
        let mut merged: Self = serde_json::from_value(current)
            .map_err(|e| ClipsaError::InvalidSetting(e.to_string()))?;
        merged.normalize();
        merged.validate()?;
        Ok(merged)
    }

    fn normalize(&mut self) {
        self.global_hotkey = self.global_hotkey.trim().to_string();
        let mut apps: Vec<String> = self
            .excluded_apps
            .iter()
            .map(|app| app.trim().to_lowercase())
            .filter(|app| !app.is_empty())
            .collect();
        apps.sort();
        apps.dedup();
        self.excluded_apps = apps;
    }

    fn validate(&self) -> ClipsaResult<()> {
        let check = |ok: bool, msg: &str| {
            if ok {
                Ok(())
            } else {
                Err(ClipsaError::InvalidSetting(msg.into()))
            }
        };
        check(
            (10..=100_000).contains(&self.max_items),
            "max_items must be between 10 and 100000",
        )?;
        check(
            self.auto_cleanup_days <= 3650,
            "auto_cleanup_days must be between 0 and 3650",
        )?;
        check(
            (10..=200).contains(&self.items_per_page),
            "items_per_page must be between 10 and 200",
        )?;
        check(
            self.lock_after_minutes <= 1440,
            "lock_after_minutes must be between 0 and 1440",
        )
    }

    pub fn is_app_excluded(&self, app: &str) -> bool {
        let app = app.to_lowercase();
        self.excluded_apps.contains(&app)
    }
}

/// Settings the update window may change while Clipsa is locked.
#[cfg(feature = "self-update")]
const UPDATER_KEYS: [&str; 3] = [
    "auto_install_updates",
    "skipped_update_version",
    "update_snoozed_until",
];

/// Whether `patch` is an object touching only the update window's settings.
#[cfg(feature = "self-update")]
pub fn is_updater_patch(patch: &serde_json::Value) -> bool {
    patch.as_object().is_some_and(|fields| {
        fields
            .keys()
            .all(|key| UPDATER_KEYS.contains(&key.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_applies_patch_and_normalizes() {
        let merged = AppSettings::default()
            .merged(json!({
                "max_items": 50,
                "excluded_apps": [" Com.Foo.App ", "", "com.foo.app", "KeePass.exe"],
                "global_hotkey": "  Alt+V "
            }))
            .unwrap();
        assert_eq!(merged.max_items, 50);
        assert_eq!(merged.excluded_apps, vec!["com.foo.app", "keepass.exe"]);
        assert_eq!(merged.global_hotkey, "Alt+V");
        assert!(merged.is_app_excluded("COM.FOO.APP"));
    }

    #[test]
    fn merge_rejects_unknown_keys_bad_types_and_out_of_range() {
        let base = AppSettings::default();
        assert!(base.merged(json!({ "require_password": true })).is_err());
        assert!(base.merged(json!({ "max_items": "many" })).is_err());
        assert!(base.merged(json!({ "max_items": 5 })).is_err());
        assert!(base.merged(json!({ "items_per_page": 500 })).is_err());
        assert!(base.merged(json!({ "theme": "neon" })).is_err());
        assert!(base.merged(json!([])).is_err());
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn updater_patch_allows_only_update_preferences() {
        assert!(is_updater_patch(&json!({
            "skipped_update_version": "1.2.0",
            "update_snoozed_until": 1_700_000_000_000u64,
            "auto_install_updates": true
        })));
        assert!(!is_updater_patch(
            &json!({ "skipped_update_version": "1.2.0", "app_lock": false })
        ));
        assert!(!is_updater_patch(&json!({ "check_for_updates": true })));
        assert!(!is_updater_patch(&json!([])));
        assert!(AppSettings::default()
            .merged(json!({ "update_snoozed_until": 1_700_000_000_000u64 }))
            .is_ok());
    }
}
