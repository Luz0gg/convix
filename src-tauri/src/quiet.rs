//! "Quiet" Explorer conversions: right-click › ConvertIO › Convertir en PNG converts in the
//! background without showing the window, then shows one Windows notification.
//!
//! Selecting 30 files makes Explorer start 30 processes. The first one becomes the worker;
//! the others forward their file through the single-instance plugin into `pending`.
//! The worker exits once nothing new has arrived for a short moment.

use crate::{
    AppState,
    cli::LaunchRequest,
    jobs::{BatchRequest, JobEvent, JobStatus, Runner},
    options,
};
use convertio_core::CancelToken;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// How long to wait for more forwarded files before finishing.
const IDLE_EXIT: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct QuietState {
    pub active: bool,
    pub pending: Vec<LaunchRequest>,
    /// Worker is finishing: new requests can no longer be queued.
    pub closed: bool,
    /// The user opened the window meanwhile: don't quit at the end.
    pub keep_open: bool,
}

/// Queue a forwarded request for the background worker. False if not in quiet mode.
pub fn enqueue(state: &AppState, req: LaunchRequest) -> bool {
    let mut q = state.quiet.lock().unwrap();
    if q.active && !q.closed && req.operation.is_some() {
        q.pending.push(req);
        true
    } else {
        false
    }
}

pub fn keep_open(state: &AppState) {
    state.quiet.lock().unwrap().keep_open = true;
}

pub fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    let mut results: Vec<JobEvent> = Vec::new();
    let mut idle_since = Instant::now();

    loop {
        let batch = {
            let mut q = state.quiet.lock().unwrap();
            if q.pending.is_empty() {
                if idle_since.elapsed() > IDLE_EXIT {
                    q.closed = true;
                    break;
                }
                Vec::new()
            } else {
                std::mem::take(&mut q.pending)
            }
        };
        if batch.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }

        let runner = Runner {
            registry: state.registry.clone(),
            modules: state.modules.clone(),
            settings: state.settings.read().unwrap().clone(),
        };
        for req in batch {
            let Some(op) = req.operation else { continue };
            let request = BatchRequest {
                batch_id: "quiet".into(),
                options: options::get(&state.options_path, &op),
                operation_id: op,
                inputs: req.files,
                output_dir: None,
                output_path: None,
            };
            results.extend(runner.run(&request, &CancelToken::new(), &|_| {}));
        }
        idle_since = Instant::now();
    }

    let lang = state.settings.read().unwrap().language.clone();
    let _ = app.notification().builder().title("ConvertIO").body(summary(&results, &lang)).show();

    if !state.quiet.lock().unwrap().keep_open {
        std::thread::sleep(Duration::from_millis(400)); // let the notification reach the OS
        app.exit(0);
    }
}

fn summary(results: &[JobEvent], lang: &str) -> String {
    let done: Vec<_> = results.iter().filter(|r| r.status == JobStatus::Done).collect();
    let failed: Vec<_> = results.iter().filter(|r| r.status == JobStatus::Failed).collect();
    let fr = lang != "en";
    let name = |r: &JobEvent| r.outputs.first().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();

    match (done.len(), failed.first()) {
        (1, None) => format!("✓ {}", name(done[0])),
        (n, None) => if fr { format!("✓ {n} fichiers convertis") } else { format!("✓ {n} files converted") },
        (n, Some(first)) => {
            let err = first.error.clone().unwrap_or_default();
            if fr {
                format!("{n} réussi(s), {} échec(s) — {err}", failed.len())
            } else {
                format!("{n} succeeded, {} failed — {err}", failed.len())
            }
        }
    }
}
