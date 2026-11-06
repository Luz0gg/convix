//! External modules: heavy tools (ffmpeg, pdfium, later upscalers…) that are not shipped
//! in the installer but downloaded on first use, verified, and updated independently.
//!
//! Layout on disk: `<app_data>/modules/<id>/<version>/…` plus `<app_data>/modules/installed.json`.
//! The list of available modules comes from a manifest (see `docs/ARCHITECTURE.md`), so a new
//! module version can ship without releasing a new app version.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One downloadable module, as described in the remote manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    /// `"ffmpeg"`, `"pdfium"`… — what operations list in `requires`.
    pub id: String,
    pub name: String,
    pub version: String,
    pub description_key: String,
    pub license: String,
    pub assets: Vec<ModuleAsset>,
}

/// The file to fetch for one platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAsset {
    /// Rust target triple, e.g. `"x86_64-pc-windows-msvc"`.
    pub target: String,
    pub url: String,
    /// Mandatory: a download whose hash does not match is deleted, never executed.
    pub sha256: String,
    pub size: u64,
    /// Path of the main executable/library inside the archive.
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ModuleStatus {
    NotInstalled,
    Installed { version: String, path: PathBuf },
    UpdateAvailable { installed: String, latest: String, path: PathBuf },
}

/// What we remember about installed modules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledIndex {
    pub modules: Vec<InstalledModule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModule {
    pub id: String,
    pub version: String,
    /// Absolute path of the entry file.
    pub path: PathBuf,
}

pub struct ModuleManager {
    root: PathBuf,
}

impl ModuleManager {
    pub fn new(app_data: &Path) -> Self {
        Self { root: app_data.join("modules") }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("installed.json")
    }

    pub fn installed(&self) -> InstalledIndex {
        std::fs::read(self.index_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Path of a module's entry file if installed and still present on disk.
    pub fn locate(&self, id: &str) -> Option<PathBuf> {
        self.installed()
            .modules
            .into_iter()
            .find(|m| m.id == id)
            .map(|m| m.path)
            .filter(|p| p.exists())
    }

    pub fn status(&self, id: &str, latest: Option<&ModuleManifest>) -> ModuleStatus {
        let installed = self.installed().modules.into_iter().find(|m| m.id == id && m.path.exists());
        match (installed, latest) {
            (None, _) => ModuleStatus::NotInstalled,
            (Some(m), Some(l)) if l.version != m.version => ModuleStatus::UpdateAvailable {
                installed: m.version,
                latest: l.version.clone(),
                path: m.path,
            },
            (Some(m), _) => ModuleStatus::Installed { version: m.version, path: m.path },
        }
    }

    // TODO(Phase 2): install(manifest, progress, cancel) — download to a temp file,
    // verify sha256, extract into <root>/<id>/<version>/, update installed.json atomically,
    // then delete older versions.
}
