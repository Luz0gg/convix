use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported or unknown file format")]
    UnsupportedFormat,

    #[error("unknown operation `{0}`")]
    UnknownOperation(String),

    #[error("invalid option `{name}`: {reason}")]
    InvalidOption { name: String, reason: String },

    /// An external module (ffmpeg, pdfium…) must be installed first.
    /// The UI catches this and offers to download the module.
    #[error("module `{0}` is required but not installed")]
    ModuleMissing(String),

    #[error("operation cancelled")]
    Cancelled,

    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),

    #[error("{0}")]
    Failed(String),
}

impl Error {
    /// Stable, language-neutral code the UI translates (`error.<code>` keys).
    /// `failed` and `io` mean "show the message as is".
    pub fn code(&self) -> String {
        match self {
            Error::Io(e) => io_code(e).to_owned(),
            Error::UnsupportedFormat => "unsupportedFormat".into(),
            Error::UnknownOperation(_) => "unknownOperation".into(),
            Error::InvalidOption { .. } => "invalidOption".into(),
            Error::ModuleMissing(id) => format!("moduleMissing:{id}"),
            Error::Cancelled => "cancelled".into(),
            Error::NotImplemented(_) => "notImplemented".into(),
            Error::Failed(_) => "failed".into(),
        }
    }
}

/// The I/O failures a user can act on: missing file, no rights, file open elsewhere, disk full.
pub fn io_code(e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind::*;
    // Windows: ERROR_SHARING_VIOLATION (32), ERROR_LOCK_VIOLATION (33) = open in another program.
    if matches!(e.raw_os_error(), Some(32 | 33)) {
        return "io.locked";
    }
    match e.kind() {
        NotFound => "io.notFound",
        PermissionDenied => "io.denied",
        StorageFull => "io.diskFull",
        _ => "io",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        assert_eq!(Error::ModuleMissing("ffmpeg".into()).code(), "moduleMissing:ffmpeg");
        assert_eq!(Error::Cancelled.code(), "cancelled");
        let locked = std::io::Error::from_raw_os_error(32);
        #[cfg(windows)]
        assert_eq!(Error::Io(locked).code(), "io.locked");
        #[cfg(not(windows))]
        let _ = locked;
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(Error::Io(missing).code(), "io.notFound");
    }
}
