//! Batch execution: runs one operation on many files in parallel (one per CPU core),
//! reports each file's progress to the UI, supports cancellation, and never lets
//! a crash on one file take down the others.

use crate::settings::Settings;
use convix_core::{
    CancelToken, Error, Format, JobContext, OptionValues, OutputPolicy, Registry, format, output,
};
use convix_modules::ModuleManager;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    sync::Arc,
};

pub const EVENT: &str = "job-event";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub batch_id: String,
    pub input: PathBuf,
    pub status: JobStatus,
    pub outputs: Vec<PathBuf>,
    pub input_size: u64,
    pub output_size: u64,
    pub error: Option<String>,
    /// 0.0 → 1.0 while running, for converters that report it (audio via FFmpeg…).
    pub progress: Option<f32>,
    /// Machine-readable error, e.g. `"moduleMissing:ffmpeg"`, so the UI can react.
    pub error_code: Option<String>,
}

pub struct BatchRequest {
    pub batch_id: String,
    pub operation_id: String,
    pub inputs: Vec<PathBuf>,
    pub options: OptionValues,
    /// Overrides the settings' output folder (picked in a dialog).
    pub output_dir: Option<PathBuf>,
    /// Exact output file (save dialog, single input only).
    pub output_path: Option<PathBuf>,
}

pub struct Runner {
    pub registry: Registry,
    pub modules: Arc<ModuleManager>,
    pub settings: Settings,
}

fn size_of(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

/// Keep the user's extension spelling for same-format results (`photo.jpeg` → `photo-min.jpeg`).
fn output_extension(input: &Path, input_format: Format, output_format: Format) -> String {
    if input_format == output_format {
        if let Some(ext) = input.extension().and_then(|e| e.to_str()) {
            return ext.to_ascii_lowercase();
        }
    }
    output_format.extension().to_owned()
}

impl Runner {
    pub fn run(
        &self,
        req: &BatchRequest,
        cancel: &CancelToken,
        emit: &(dyn Fn(JobEvent) + Sync),
    ) -> Vec<JobEvent> {
        let policy = match (&req.output_dir, &self.settings.output) {
            (Some(dir), _) => OutputPolicy::FixedFolder { path: dir.clone() },
            (None, OutputPolicy::AskEachTime) => OutputPolicy::NextToSource,
            (None, p) => p.clone(),
        };
        // Merge PDFs, images → PDF…: one job takes every file and makes one result.
        if self.is_combined(req) {
            let explicit = req.output_path.as_deref();
            return vec![self.run_job(req, &req.inputs, &policy, explicit, cancel, emit)];
        }

        let explicit = req.output_path.as_deref().filter(|_| req.inputs.len() == 1);
        req.inputs
            .par_iter()
            .map(|input| {
                self.run_job(
                    req,
                    std::slice::from_ref(input),
                    &policy,
                    explicit,
                    cancel,
                    emit,
                )
            })
            .collect()
    }

    pub fn is_combined(&self, req: &BatchRequest) -> bool {
        let Some(first) = req.inputs.first() else {
            return false;
        };
        format::detect(first)
            .ok()
            .flatten()
            .and_then(|f| self.registry.describe(f, &req.operation_id))
            .is_some_and(|op| op.multi_input)
    }

    /// One job: one file, or all the files of a combining operation (reported under the first).
    fn run_job(
        &self,
        req: &BatchRequest,
        inputs: &[PathBuf],
        policy: &OutputPolicy,
        explicit: Option<&Path>,
        cancel: &CancelToken,
        emit: &(dyn Fn(JobEvent) + Sync),
    ) -> JobEvent {
        let input = &inputs[0];
        let base = JobEvent {
            batch_id: req.batch_id.clone(),
            input: input.clone(),
            status: JobStatus::Running,
            outputs: Vec::new(),
            input_size: inputs.iter().map(|p| size_of(p)).sum(),
            output_size: 0,
            error: None,
            progress: None,
            error_code: None,
        };
        if cancel.is_cancelled() {
            let ev = JobEvent {
                status: JobStatus::Cancelled,
                ..base
            };
            emit(ev.clone());
            return ev;
        }
        emit(base.clone());

        // Live progress, throttled to whole percents so the UI isn't flooded.
        let last_percent = std::sync::atomic::AtomicI32::new(-1);
        let on_progress = |f: f32| {
            let percent = (f * 100.0) as i32;
            if last_percent.swap(percent, std::sync::atomic::Ordering::Relaxed) != percent {
                emit(JobEvent {
                    progress: Some(f),
                    ..base.clone()
                });
            }
        };

        let ev = match self.run_one(req, inputs, policy, explicit, cancel, &on_progress) {
            Ok(outputs) => JobEvent {
                status: JobStatus::Done,
                output_size: outputs.iter().map(|p| size_of(p)).sum(),
                outputs,
                ..base
            },
            Err(Error::Cancelled) => JobEvent {
                status: JobStatus::Cancelled,
                ..base
            },
            Err(Error::ModuleMissing(id)) => JobEvent {
                status: JobStatus::Failed,
                error: Some(format!("le module {id} n'est pas installé")),
                error_code: Some(format!("moduleMissing:{id}")),
                ..base
            },
            Err(e) => JobEvent {
                status: JobStatus::Failed,
                error: Some(e.to_string()),
                ..base
            },
        };
        if ev.status == JobStatus::Failed {
            crate::log::error(&format!("{} | {} | {}", req.operation_id, input.display(), ev.error.as_deref().unwrap_or("?")));
        }
        emit(ev.clone());
        ev
    }

    fn run_one(
        &self,
        req: &BatchRequest,
        inputs: &[PathBuf],
        policy: &OutputPolicy,
        explicit: Option<&Path>,
        cancel: &CancelToken,
        on_progress: &(dyn Fn(f32) + Sync),
    ) -> convix_core::Result<Vec<PathBuf>> {
        let input = inputs[0].as_path();
        let in_fmt = format::detect(input)?.ok_or(Error::UnsupportedFormat)?;
        let op = self
            .registry
            .describe(in_fmt, &req.operation_id)
            .ok_or_else(|| {
                Error::Failed(format!(
                    "action indisponible pour un fichier {}",
                    in_fmt.label()
                ))
            })?;
        // Before reserving an output name: no empty file left behind for a missing module.
        if let Some(missing) = op
            .requires
            .iter()
            .find(|id| self.modules.locate(id).is_none())
        {
            return Err(Error::ModuleMissing((*missing).to_owned()));
        }
        let out_fmt = op.output.unwrap_or(in_fmt);

        let out = match explicit {
            Some(p) => p.to_path_buf(),
            None => {
                let ext = output_extension(input, in_fmt, out_fmt);
                output::reserve(output::target_path(input, &ext, op.output_suffix, policy))?
            }
        };

        let locate = |id: &str| self.modules.locate(id);
        let ctx = JobContext {
            operation_id: &req.operation_id,
            inputs,
            output: out.clone(),
            options: &req.options,
            progress: &|f: f32| on_progress(f),
            cancel,
            modules: &locate,
        };
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| self.registry.run(&ctx)))
            .unwrap_or_else(|_| Err(Error::Failed("erreur interne pendant la conversion".into())));
        if result.is_err() {
            output::discard(&out);
        }
        result
    }
}
