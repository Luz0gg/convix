//! External modules: heavy tools (ffmpeg, pdfium, later upscalers…) that are not shipped
//! in the installer but downloaded on first use, verified, and updated independently.
//!
//! Layout on disk: `<app_data>/modules/<id>/<version>/…` plus `<app_data>/modules/installed.json`.
//! The list of available modules is `manifest.json`, compiled into the app: a new FFmpeg
//! version ships with an app update (which is automatic).

mod install;

pub use install::{InstallPhase, InstallProgress};

use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

/// One downloadable module, as described in `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    /// `"ffmpeg"`, `"pdfium"`… — what operations list in `requires`.
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub homepage: String,
    pub assets: Vec<ModuleAsset>,
}

impl ModuleManifest {
    /// The download for this machine, if the module supports it.
    pub fn asset(&self) -> Option<&ModuleAsset> {
        // Windows on ARM runs the x64 build through its built-in emulation.
        let wanted = [current_target(), "x86_64-pc-windows-msvc"];
        wanted.iter().find_map(|t| self.assets.iter().find(|a| a.target == *t))
    }
}

/// The file to fetch for one platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAsset {
    /// Rust target triple, e.g. `"x86_64-pc-windows-msvc"`.
    pub target: String,
    pub url: String,
    /// Mandatory: a download whose hash does not match is deleted, never extracted or run.
    pub sha256: String,
    pub size: u64,
    /// Archive type: `"7z"` for now.
    pub archive: String,
    /// Path of the main executable/library inside the archive.
    pub entry: String,
    /// Other files to keep from the archive (everything else is skipped).
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ModuleStatus {
    NotInstalled,
    #[serde(rename_all = "camelCase")]
    Installed { version: String, path: PathBuf },
    #[serde(rename_all = "camelCase")]
    UpdateAvailable { installed: String, latest: String, path: PathBuf },
}

/// What the UI shows for a module.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub homepage: String,
    /// Download size in bytes (0 if not available for this machine).
    pub download_size: u64,
    pub status: ModuleStatus,
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

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("module inconnu : {0}")]
    Unknown(String),
    #[error("ce module n'est pas disponible pour cet ordinateur")]
    NoAsset,
    #[error("téléchargement impossible : {0}")]
    Network(String),
    #[error("le fichier téléchargé est corrompu ou a été modifié (empreinte SHA-256 incorrecte) : il a été supprimé")]
    Checksum,
    #[error("archive illisible : {0}")]
    Archive(String),
    #[error("installation annulée")]
    Cancelled,
    #[error("erreur disque : {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = ModuleError> = std::result::Result<T, E>;

pub const fn current_target() -> &'static str {
    if cfg!(all(windows, target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(windows, target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else {
        "unsupported"
    }
}

/// The modules this version of Convix knows about.
pub fn builtin_manifest() -> Vec<ModuleManifest> {
    serde_json::from_str(include_str!("../manifest.json")).expect("manifest.json is valid")
}

pub struct ModuleManager {
    root: PathBuf,
    manifest: Vec<ModuleManifest>,
    /// Serialises writes to `installed.json`.
    index_lock: Mutex<()>,
}

impl ModuleManager {
    pub fn new(app_data: &Path) -> Self {
        Self::with_manifest(app_data.join("modules"), builtin_manifest())
    }

    pub fn with_manifest(root: PathBuf, manifest: Vec<ModuleManifest>) -> Self {
        Self { root, manifest, index_lock: Mutex::new(()) }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self, id: &str) -> Option<&ModuleManifest> {
        self.manifest.iter().find(|m| m.id == id)
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

    fn write_index(&self, index: &InstalledIndex) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let tmp = self.root.join("installed.json.part");
        std::fs::write(&tmp, serde_json::to_vec_pretty(index).expect("serializable"))?;
        std::fs::rename(&tmp, self.index_path())?;
        Ok(())
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

    pub fn status(&self, id: &str) -> ModuleStatus {
        let installed = self.installed().modules.into_iter().find(|m| m.id == id && m.path.exists());
        match (installed, self.manifest(id)) {
            (None, _) => ModuleStatus::NotInstalled,
            (Some(m), Some(l)) if l.version != m.version => ModuleStatus::UpdateAvailable {
                installed: m.version,
                latest: l.version.clone(),
                path: m.path,
            },
            (Some(m), _) => ModuleStatus::Installed { version: m.version, path: m.path },
        }
    }

    pub fn list(&self) -> Vec<ModuleInfo> {
        self.manifest
            .iter()
            .map(|m| ModuleInfo {
                id: m.id.clone(),
                name: m.name.clone(),
                version: m.version.clone(),
                license: m.license.clone(),
                homepage: m.homepage.clone(),
                download_size: m.asset().map_or(0, |a| a.size),
                status: self.status(&m.id),
            })
            .collect()
    }

    /// Download, verify and unpack a module, then record it. Older versions are removed.
    pub fn install(
        &self,
        id: &str,
        progress: &dyn Fn(InstallProgress),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<InstalledModule> {
        let manifest = self.manifest(id).ok_or_else(|| ModuleError::Unknown(id.to_owned()))?;
        let asset = manifest.asset().ok_or(ModuleError::NoAsset)?;
        let dir = self.root.join(&manifest.id);
        std::fs::create_dir_all(&dir)?;

        let archive = dir.join("download.part");
        let downloaded = install::download(&asset.url, &archive, asset.size, progress, cancelled)
            .and_then(|()| install::verify(&archive, asset, progress));
        if let Err(e) = downloaded {
            let _ = std::fs::remove_file(&archive);
            return Err(e);
        }
        let installed = self.unpack_and_record(manifest, asset, &archive, progress, cancelled);
        let _ = std::fs::remove_file(&archive);
        installed
    }

    /// Install from an archive already on disk (verified like a download). Used by tests.
    pub fn install_from_file(&self, id: &str, archive: &Path) -> Result<InstalledModule> {
        let manifest = self.manifest(id).ok_or_else(|| ModuleError::Unknown(id.to_owned()))?;
        let asset = manifest.asset().ok_or(ModuleError::NoAsset)?;
        install::verify(archive, asset, &|_| {})?;
        self.unpack_and_record(manifest, asset, archive, &|_| {}, &|| false)
    }

    fn unpack_and_record(
        &self,
        manifest: &ModuleManifest,
        asset: &ModuleAsset,
        archive: &Path,
        progress: &dyn Fn(InstallProgress),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<InstalledModule> {
        let dir = self.root.join(&manifest.id);
        let target = dir.join(&manifest.version);
        let staging = dir.join(format!("{}.part", manifest.version));
        let _ = std::fs::remove_dir_all(&staging);

        let entry = match install::extract(archive, asset, &staging, progress, cancelled) {
            Ok(entry) => entry,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(e);
            }
        };
        let _ = std::fs::remove_dir_all(&target);
        std::fs::rename(&staging, &target)?;

        let module = InstalledModule {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            path: target.join(entry),
        };
        {
            let _guard = self.index_lock.lock().unwrap_or_else(|e| e.into_inner());
            let mut index = self.installed();
            index.modules.retain(|m| m.id != module.id);
            index.modules.push(module.clone());
            self.write_index(&index)?;
        }
        // Old versions are no longer referenced: free the disk space.
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                if e.path() != target && e.path().is_dir() {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        Ok(module)
    }

    /// Uninstall a module (files and index entry).
    pub fn remove(&self, id: &str) -> Result<()> {
        let _guard = self.index_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut index = self.installed();
        index.modules.retain(|m| m.id != id);
        self.write_index(&index)?;
        let dir = self.root.join(id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("convix-modules-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn builtin_manifest_has_ffmpeg_for_windows() {
        let m = builtin_manifest();
        let ff = m.iter().find(|m| m.id == "ffmpeg").expect("ffmpeg listed");
        let asset = ff.asset().expect("windows asset");
        assert_eq!(asset.sha256.len(), 64);
        assert!(asset.url.starts_with("https://"));
    }

    #[test]
    fn tampered_archive_is_rejected_and_nothing_installed() {
        let root = temp_root("tampered");
        let mgr = ModuleManager::with_manifest(root.clone(), builtin_manifest());
        std::fs::create_dir_all(&root).unwrap();
        let fake = root.join("fake.7z");
        std::fs::write(&fake, b"not ffmpeg").unwrap();
        assert!(matches!(mgr.install_from_file("ffmpeg", &fake), Err(ModuleError::Checksum)));
        assert_eq!(mgr.status("ffmpeg"), ModuleStatus::NotInstalled);
        assert!(mgr.locate("ffmpeg").is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    /// Unpacks the real FFmpeg archive when it is available locally:
    /// `CONVIX_FFMPEG_7Z=…\ffmpeg-9.0.2-essentials_build.7z cargo test -p convix-modules`
    #[test]
    fn installs_real_ffmpeg_archive() {
        let Some(archive) = std::env::var_os("CONVIX_FFMPEG_7Z").map(PathBuf::from) else {
            eprintln!("skipped: set CONVIX_FFMPEG_7Z");
            return;
        };
        let root = temp_root("real");
        let mgr = ModuleManager::with_manifest(root.clone(), builtin_manifest());
        let installed = mgr.install_from_file("ffmpeg", &archive).expect("install");
        assert!(installed.path.ends_with("ffmpeg.exe"));
        assert_eq!(mgr.locate("ffmpeg").as_deref(), Some(installed.path.as_path()));
        assert!(installed.path.with_file_name("LICENSE").exists());
        assert!(!installed.path.with_file_name("ffprobe.exe").exists(), "only the listed files are kept");
        mgr.remove("ffmpeg").unwrap();
        assert!(mgr.locate("ffmpeg").is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    /// Real network download (35 MB): `cargo test -p convix-modules -- --ignored`
    #[test]
    #[ignore]
    fn downloads_ffmpeg_for_real() {
        let root = temp_root("net");
        let mgr = ModuleManager::with_manifest(root.clone(), builtin_manifest());
        let installed = mgr.install("ffmpeg", &|_| {}, &|| false).expect("install");
        assert!(installed.path.exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
