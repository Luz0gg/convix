//! User settings, persisted as JSON in the app config folder.
//! Kept on the Rust side (not in the webview) because the Explorer context menu
//! needs them too (output folder, menu language).

use convix_core::OutputPolicy;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output: OutputPolicy,
    /// "fr" | "en"
    pub language: String,
    /// "system" | "dark" | "light"
    pub theme: String,
    /// Explorer context menu enabled.
    pub shell_integration: bool,
    /// Context-menu conversions run in the background (notification only, no window).
    pub quiet_context_menu: bool,
    /// Look for a new version at startup (GitHub Releases, signature checked).
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output: OutputPolicy::NextToSource,
            language: "fr".into(),
            theme: "system".into(),
            shell_integration: true,
            quiet_context_menu: true,
            check_updates: true,
        }
    }
}

impl Settings {
    /// Unknown values (hand-edited file, newer version) fall back to the defaults:
    /// the UI must never receive a language or theme it can't display.
    pub fn sanitized(mut self) -> Self {
        if !matches!(self.language.as_str(), "fr" | "en") {
            self.language = Self::default().language;
        }
        if !matches!(self.theme.as_str(), "system" | "dark" | "light") {
            self.theme = Self::default().theme;
        }
        self
    }
}

pub fn load(path: &Path) -> Settings {
    crate::store::load::<Settings>(path).sanitized()
}

pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    crate::store::save(path, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_values_fall_back() {
        let s: Settings =
            serde_json::from_str(r#"{"language":"de","theme":"neon","checkUpdates":false}"#).unwrap();
        let s = s.sanitized();
        assert_eq!(
            (s.language.as_str(), s.theme.as_str(), s.check_updates),
            ("fr", "system", false)
        );
    }
}
