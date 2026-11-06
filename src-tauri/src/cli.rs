//! Command line: `Convix.exe [--op <operation-id>] <file>...`
//!
//! Used by the Explorer context menu ("Convix › Convertir en PNG" runs
//! `Convix.exe --op image.convert.png "C:\…\photo.webp"`). Without `--op`, the files are
//! simply opened in the window.
//!
//! Arguments are read as `OsString`: a file name that isn't valid Unicode (possible on
//! Windows) must not crash the app at startup like `std::env::args()` would.

use serde::Serialize;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub operation: Option<String>,
    pub files: Vec<PathBuf>,
}

pub fn parse<I, A>(args: I, cwd: Option<&Path>) -> Option<LaunchRequest>
where
    I: IntoIterator<Item = A>,
    A: Into<OsString>,
{
    let mut operation = None;
    let mut files = Vec::new();
    let mut args = args.into_iter().map(Into::into);
    while let Some(arg) = args.next() {
        if arg == "--op" {
            operation = args
                .next()
                .map(|op| op.to_string_lossy().into_owned())
                .filter(|op| !op.is_empty());
        } else if arg.to_string_lossy().starts_with("--") {
            // Unknown flag (or one added by Windows/WebView2): ignored.
        } else if !arg.is_empty() {
            let p = PathBuf::from(arg);
            files.push(match cwd {
                Some(cwd) if p.is_relative() => cwd.join(p),
                _ => p,
            });
        }
    }
    (!files.is_empty()).then_some(LaunchRequest { operation, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str], cwd: Option<&str>) -> Option<LaunchRequest> {
        parse(args.iter().copied(), cwd.map(Path::new))
    }

    #[test]
    fn operation_and_files() {
        let r = p(&["--op", "image.convert.png", r"C:\a\été 1.webp", r"D:\b.jpg"], None).unwrap();
        assert_eq!(r.operation.as_deref(), Some("image.convert.png"));
        assert_eq!(r.files, [PathBuf::from(r"C:\a\été 1.webp"), PathBuf::from(r"D:\b.jpg")]);
    }

    #[test]
    fn files_only_and_relative() {
        let r = p(&["photo.png"], Some(r"C:\dossier")).unwrap();
        assert_eq!(r.operation, None);
        assert_eq!(r.files, [Path::new(r"C:\dossier").join("photo.png")]);
    }

    #[test]
    fn nothing_to_do() {
        assert_eq!(p(&[], None), None);
        assert_eq!(p(&["--op", "image.convert.png"], None), None);
        assert_eq!(p(&["--unknown-flag", ""], None), None);
    }

    #[test]
    fn op_last_without_value() {
        let r = p(&[r"C:\a.png", "--op"], None).unwrap();
        assert_eq!(r.operation, None);
        assert_eq!(r.files.len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_name_does_not_panic() {
        use std::os::windows::ffi::OsStringExt;
        // Lone surrogate: valid on NTFS, invalid UTF-8/UTF-16 text.
        let bad = OsString::from_wide(&[0x43, 0x3a, 0x5c, 0xD800, 0x2e, 0x70, 0x6e, 0x67]);
        let r = parse([OsString::from("--op"), OsString::from("image.convert.png"), bad.clone()], None).unwrap();
        assert_eq!(r.files, [PathBuf::from(bad)]);
    }
}
