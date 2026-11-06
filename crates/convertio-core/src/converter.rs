use crate::{Format, JobContext, OperationDescriptor, Result};
use std::path::PathBuf;

/// A family of conversions (images, audio, PDF…). Each converter crate implements one.
///
/// Converters must be cheap to construct and stateless: one instance serves every job,
/// possibly from several threads at once.
pub trait Converter: Send + Sync {
    /// Short stable id, used as the operation id prefix (`"image"`, `"audio"`, `"pdf"`).
    fn id(&self) -> &'static str;

    /// Everything this converter can do with a file of `input` format.
    /// Return an empty list when the format is not handled.
    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor>;

    /// Run one operation. `ctx.output` is the final path to write (already de-duplicated).
    /// Returns the files actually written (may be several, e.g. PDF → one PNG per page).
    fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>>;
}
