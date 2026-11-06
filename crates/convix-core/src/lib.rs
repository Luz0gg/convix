//! Convix engine.
//!
//! Everything the UI can do goes through this crate:
//! 1. [`format::detect`] identifies a dropped file (magic bytes first, extension as fallback).
//! 2. [`Registry::operations_for`] asks every registered [`Converter`] what it can do with it.
//! 3. The UI renders those [`OperationDescriptor`]s (and their options) without knowing
//!    anything about images, audio or PDF.
//! 4. [`Registry::run`] dispatches the chosen operation to the converter that declared it.
//!
//! Adding a conversion = adding an operation to a converter crate. No UI change needed.

pub mod converter;
pub mod error;
pub mod format;
pub mod job;
pub mod operation;
pub mod output;
pub mod registry;

pub use converter::Converter;
pub use error::{Error, Result};
pub use format::{Format, MediaKind};
pub use job::{CancelToken, JobContext, ProgressSink};
pub use operation::{OperationCategory, OperationDescriptor, OptionSpec, OptionValues};
pub use output::OutputPolicy;
pub use registry::Registry;
