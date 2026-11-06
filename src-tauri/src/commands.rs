//! Commands callable from the UI (`invoke("name", args)` in `src/lib/api.ts`).
//! Errors are sent as strings for now; a structured error type comes with Phase 1.

use crate::{AppState, cli::LaunchRequest, shell_integration};
use convertio_core::{
    CancelToken, Format, JobContext, MediaKind, OperationDescriptor, OptionValues, OutputPolicy,
    format, output,
};
use convertio_modules::ModuleStatus;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION") }
}

/// Returns (once) the files passed on the command line at startup.
#[tauri::command]
pub fn take_launch_request(state: State<'_, AppState>) -> Option<LaunchRequest> {
    state.pending_launch.lock().ok()?.take()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    path: PathBuf,
    name: String,
    size: u64,
    format: Option<Format>,
    format_label: Option<&'static str>,
    kind: Option<MediaKind>,
    operations: Vec<OperationDescriptor>,
    error: Option<String>,
}

/// Detect each dropped file and list what can be done with it.
#[tauri::command]
pub fn inspect_files(state: State<'_, AppState>, paths: Vec<PathBuf>) -> Vec<FileInfo> {
    paths
        .into_iter()
        .map(|path| {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let (format, error) = match format::detect(&path) {
                Ok(f) => (f, None),
                Err(e) => (None, Some(e.to_string())),
            };
            FileInfo {
                operations: format.map(|f| state.registry.operations_for(f)).unwrap_or_default(),
                format_label: format.map(Format::label),
                kind: format.map(Format::kind),
                format,
                path,
                name,
                size,
                error,
            }
        })
        .collect()
}

/// Run one operation. Heavy work runs on a blocking thread so the UI stays fluid.
#[tauri::command]
pub async fn run_operation(
    state: State<'_, AppState>,
    operation_id: String,
    inputs: Vec<PathBuf>,
    options: OptionValues,
    output_ext: String,
) -> Result<Vec<PathBuf>, String> {
    let registry = state.registry.clone();
    let modules = state.modules.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let first = inputs.first().ok_or("no input file")?;
        // TODO(Phase 1): read the policy from user settings.
        let out = output::resolve(first, &output_ext, &OutputPolicy::NextToSource, None);
        let cancel = CancelToken::new();
        let progress = |_f: f32| {}; // TODO(Phase 1): emit "job-progress" events
        let locate = |id: &str| modules.locate(id);
        let ctx = JobContext {
            operation_id: &operation_id,
            inputs: &inputs,
            output: out,
            options: &options,
            progress: &progress,
            cancel: &cancel,
            modules: &locate,
        };
        registry.run(&ctx).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn module_status(state: State<'_, AppState>, id: String) -> ModuleStatus {
    state.modules.status(&id, None)
}

#[tauri::command]
pub fn set_shell_integration(enabled: bool) -> Result<(), String> {
    if enabled { shell_integration::install() } else { shell_integration::uninstall() }
}
