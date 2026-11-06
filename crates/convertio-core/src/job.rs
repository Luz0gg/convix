use crate::{Error, OptionValues, Result};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Shared flag a running job polls to stop early.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    /// `?`-friendly check to sprinkle in long loops.
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() { Err(Error::Cancelled) } else { Ok(()) }
    }
}

/// Receives progress updates (0.0 → 1.0). The Tauri shell forwards them to the UI.
pub trait ProgressSink: Send + Sync {
    fn progress(&self, fraction: f32);
}

impl<F: Fn(f32) + Send + Sync> ProgressSink for F {
    fn progress(&self, fraction: f32) {
        self(fraction)
    }
}

/// Everything a converter needs to run one operation.
pub struct JobContext<'a> {
    pub operation_id: &'a str,
    /// One file for most operations, several for `multi_input` ones.
    pub inputs: &'a [PathBuf],
    /// Where to write. Already resolved by [`crate::output`] (never overwrites).
    pub output: PathBuf,
    pub options: &'a OptionValues,
    pub progress: &'a dyn ProgressSink,
    pub cancel: &'a CancelToken,
    /// Resolved paths of the external modules the operation declared in `requires`.
    pub modules: &'a dyn Fn(&str) -> Option<PathBuf>,
}

impl JobContext<'_> {
    pub fn module(&self, id: &str) -> Result<PathBuf> {
        (self.modules)(id).ok_or_else(|| Error::ModuleMissing(id.to_owned()))
    }
}
