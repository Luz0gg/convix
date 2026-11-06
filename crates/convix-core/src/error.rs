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
