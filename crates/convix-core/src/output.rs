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

/// A fresh temporary file next to `target` (`<target>.<n>.part`), created exclusively:
/// it can never be — and never overwrite — a file that already exists.
/// Same folder as the target, so the final rename never crosses drives.
pub fn temp_beside(target: &Path) -> Result<PathBuf> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    for _ in 0..1000 {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name = target.as_os_str().to_owned();
        name.push(format!(".{}-{n}.part", std::process::id()));
        let candidate = PathBuf::from(name);
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(_) => return Ok(candidate),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(io::Error::other("no free temporary file name").into())
}

/// Move a finished temporary file onto `target`, replacing the reserved placeholder.
/// Retries a few times: right after a file is created, antivirus or the search indexer
/// may hold it open for a moment and Windows refuses the rename ("access denied").
pub fn commit(tmp: &Path, target: &Path) -> Result<()> {
    let mut last = None;
    for attempt in 0..5 {
        match std::fs::rename(tmp, target) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied || matches!(e.raw_os_error(), Some(32 | 33)) => {
                last = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(100 << attempt));
            }
            Err(e) => {
                last = Some(e);
                break;
            }
        }
    }
    let _ = std::fs::remove_file(tmp);
    Err(last.expect("at least one attempt").into())
}

/// Write `bytes` to `target` without ever leaving a half-written file:
/// write to a fresh temporary file, flush it to disk, then rename it over `target`.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = temp_beside(target)?;
    let written = (|| {
        let mut f = OpenOptions::new().write(true).truncate(true).open(&tmp)?;
        io::Write::write_all(&mut f, bytes)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    commit(&tmp, target)
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
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no temporary file left");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn existing_part_file_is_never_touched() {
        let dir = temp_dir("part");
        // A user file that happens to have the old temporary name.
        std::fs::write(dir.join("out.png.part"), b"precious").unwrap();
        let target = reserve(dir.join("out.png")).unwrap();
        write_atomic(&target, b"data").unwrap();
        assert_eq!(std::fs::read(dir.join("out.png.part")).unwrap(), b"precious");
        assert_eq!(std::fs::read(&target).unwrap(), b"data");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn failed_write_leaves_nothing_behind() {
        let dir = temp_dir("fail");
        let target = dir.join("missing-folder").join("out.png");
        assert!(write_atomic(&target, b"data").is_err());
        assert!(!dir.join("missing-folder").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
