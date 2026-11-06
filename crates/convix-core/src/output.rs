//! Where converted files go, and how they are written safely.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
};

/// User setting. Default: next to the original file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum OutputPolicy {
    #[default]
    NextToSource,
    FixedFolder { path: PathBuf },
    /// The UI shows a save dialog and passes the chosen path/folder explicitly.
    /// When nothing is passed (e.g. Explorer context menu), behaves like `NextToSource`.
    AskEachTime,
}

/// Target file name for `input` converted to `extension`, before de-duplication.
/// `suffix` distinguishes same-format results: `photo.png` → `photo-min.png`.
pub fn target_path(input: &Path, extension: &str, suffix: Option<&str>, policy: &OutputPolicy) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let dir = match policy {
        OutputPolicy::FixedFolder { path } => path.clone(),
        _ => input.parent().map(Path::to_path_buf).unwrap_or_default(),
    };
    dir.join(format!("{stem}{}.{extension}", suffix.unwrap_or("")))
}

/// Atomically claim a free file name: `photo.png`, else `photo (1).png`, `photo (2).png`…
///
/// An empty placeholder file is created, so two conversions running in parallel can never
/// pick the same name. The converter then replaces it via [`write_atomic`].
/// Callers must delete the placeholder if the conversion fails ([`discard`]).
pub fn reserve(path: PathBuf) -> Result<PathBuf> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();

    for i in 0u32.. {
        let candidate = match (i, &ext) {
            (0, _) => path.clone(),
            (_, Some(e)) => dir.join(format!("{stem} ({i}).{e}")),
            (_, None) => dir.join(format!("{stem} ({i})")),
        };
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(_) => return Ok(candidate),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}

/// Write `bytes` to `target` without ever leaving a half-written file:
/// write to `<target>.part`, then rename over `target`.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".part");
    let tmp = PathBuf::from(tmp);
    let result = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, target));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(result?)
}

/// Remove a reserved placeholder after a failed conversion (only if still empty).
pub fn discard(placeholder: &Path) {
    if std::fs::metadata(placeholder).is_ok_and(|m| m.len() == 0) {
        let _ = std::fs::remove_file(placeholder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("convix-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn never_overwrites() {
        let dir = temp_dir("reserve");
        let src = dir.join("photo.webp");
        std::fs::write(&src, b"x").unwrap();

        let target = target_path(&src, "png", None, &OutputPolicy::NextToSource);
        assert_eq!(target, dir.join("photo.png"));
        assert_eq!(reserve(target.clone()).unwrap(), dir.join("photo.png"));
        assert_eq!(reserve(target.clone()).unwrap(), dir.join("photo (1).png"));
        assert_eq!(reserve(target).unwrap(), dir.join("photo (2).png"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn suffix_and_fixed_folder() {
        let out = PathBuf::from("D:/exports");
        let p = target_path(Path::new("C:/a/photo.jpeg"), "jpeg", Some("-min"), &OutputPolicy::FixedFolder { path: out.clone() });
        assert_eq!(p, out.join("photo-min.jpeg"));
    }

    #[test]
    fn atomic_write_replaces_placeholder() {
        let dir = temp_dir("atomic");
        let target = reserve(dir.join("out.png")).unwrap();
        write_atomic(&target, b"data").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"data");
        assert!(!dir.join("out.png.part").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
