//! Command line: `ConvertIO.exe [--op <operation-id>] <file>...`
//!
//! Used by the Explorer context menu ("ConvertIO › Convertir en PNG" runs
//! `ConvertIO.exe --op image.convert.png "C:\…\photo.webp"`). Without `--op`, the files are
//! simply opened in the window.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub operation: Option<String>,
    pub files: Vec<PathBuf>,
}

pub fn parse(args: impl IntoIterator<Item = String>, cwd: Option<&Path>) -> Option<LaunchRequest> {
    let mut operation = None;
    let mut files = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--op" {
            operation = args.next();
        } else if !arg.starts_with("--") {
            let p = PathBuf::from(arg);
            files.push(match cwd {
                Some(cwd) if p.is_relative() => cwd.join(p),
                _ => p,
            });
        }
    }
    (!files.is_empty()).then_some(LaunchRequest { operation, files })
}
