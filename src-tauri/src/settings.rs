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

pub fn load(path: &Path) -> Settings {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.part");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)
}
