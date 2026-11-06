//! Where converted files go.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// User setting. Default: next to the original file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum OutputPolicy {
    #[default]
    NextToSource,
    FixedFolder { path: PathBuf },
    /// The UI shows a save dialog and passes the chosen path explicitly.
    AskEachTime,
}

/// Compute the target path for `input` converted to `extension`, never overwriting
/// an existing file. `explicit` is the path picked in a save dialog, if any.
pub fn resolve(input: &Path, extension: &str, policy: &OutputPolicy, explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    let stem = input.file_stem().unwrap_or_default();
    let dir = match policy {
        OutputPolicy::FixedFolder { path } => path.clone(),
        _ => input.parent().map(Path::to_path_buf).unwrap_or_default(),
    };
    let mut candidate = dir.join(stem);
    candidate.set_extension(extension);
    unique(candidate)
}

/// `photo.png` → `photo (1).png` → `photo (2).png`… until the name is free.
pub fn unique(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    (1..)
        .map(|i| {
            let name = match &ext {
                Some(e) => format!("{stem} ({i}).{e}"),
                None => format!("{stem} ({i})"),
            };
            dir.join(name)
        })
        .find(|p| !p.exists())
        .expect("infinite iterator")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_overwrites() {
        let dir = std::env::temp_dir().join(format!("convertio-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("photo.webp");
        std::fs::write(&src, b"x").unwrap();

        let first = resolve(&src, "png", &OutputPolicy::NextToSource, None);
        assert_eq!(first, dir.join("photo.png"));

        std::fs::write(&first, b"x").unwrap();
        let second = resolve(&src, "png", &OutputPolicy::NextToSource, None);
        assert_eq!(second, dir.join("photo (1).png"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
