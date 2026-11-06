//! Commands callable from the UI (`invoke("name", args)` in `src/lib/api.ts`).

use crate::{
    AppState,
    cli::LaunchRequest,
    jobs::{self, BatchRequest, JobEvent, Runner},
    settings::{self, Settings},
    shell_integration,
};
use convertio_core::{CancelToken, Format, MediaKind, OperationDescriptor, OptionValues, format};
use convertio_modules::ModuleStatus;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

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

// ---------- settings ----------

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings, String> {
    let previous = state.settings.read().unwrap().clone();
    settings::save(&state.settings_path, &settings).map_err(|e| e.to_string())?;
    *state.settings.write().unwrap() = settings.clone();

    let menu_changed = previous.shell_integration != settings.shell_integration
        || (settings.shell_integration && previous.language != settings.language);
    if menu_changed {
        if settings.shell_integration {
            shell_integration::install(&state.registry, &settings.language)?;
        } else {
            shell_integration::uninstall()?;
        }
    }
    Ok(settings)
}

// ---------- files ----------

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

const MAX_FILES_FROM_FOLDER: usize = 5000;

/// Dropped folders are expanded recursively to the files we know.
fn expand(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
        if depth > 8 || out.len() >= MAX_FILES_FROM_FOLDER {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out, depth + 1);
            } else if format::detect_extension(&p).is_some() && out.len() < MAX_FILES_FROM_FOLDER {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() { walk(&p, &mut out, 0) } else { out.push(p) }
    }
    out
}

/// Detect each dropped file and list what can be done with it.
#[tauri::command]
pub async fn inspect_files(state: State<'_, AppState>, paths: Vec<PathBuf>) -> Result<Vec<FileInfo>, String> {
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || {
        expand(paths)
            .into_iter()
            .map(|path| {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let (format, error) = match format::detect(&path) {
                    Ok(f) => (f, None),
                    Err(e) => (None, Some(e.to_string())),
                };
                FileInfo {
                    operations: format.map(|f| registry.operations_for(f)).unwrap_or_default(),
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
    })
    .await
    .map_err(|e| e.to_string())
}

// ---------- jobs ----------

/// Run one operation on many files. Per-file progress arrives as `job-event` events;
/// the final state of every file is also returned.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn run_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
    operation_id: String,
    inputs: Vec<PathBuf>,
    options: OptionValues,
    output_dir: Option<PathBuf>,
    output_path: Option<PathBuf>,
) -> Result<Vec<JobEvent>, String> {
    let cancel = CancelToken::new();
    state.batches.lock().unwrap().insert(batch_id.clone(), cancel.clone());

    let runner = Runner {
        registry: state.registry.clone(),
        modules: state.modules.clone(),
        settings: state.settings.read().unwrap().clone(),
    };
    let req = BatchRequest { batch_id: batch_id.clone(), operation_id, inputs, options, output_dir, output_path };
    let token = cancel.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        runner.run(&req, &token, &|ev| {
            let _ = app.emit(jobs::EVENT, ev);
        })
    })
    .await
    .map_err(|e| e.to_string());

    state.batches.lock().unwrap().remove(&batch_id);
    result
}

#[tauri::command]
pub fn cancel_batch(state: State<'_, AppState>, batch_id: String) {
    if let Some(token) = state.batches.lock().unwrap().get(&batch_id) {
        token.cancel();
    }
}

/// Small preview image (raw JPEG/PNG bytes → ArrayBuffer in JS).
/// Runs in a 3-thread pool so 200 dropped photos don't decode all at once.
#[tauri::command]
pub async fn thumbnail(path: PathBuf) -> Result<tauri::ipc::Response, String> {
    static POOL: std::sync::OnceLock<rayon::ThreadPool> = std::sync::OnceLock::new();
    tauri::async_runtime::spawn_blocking(move || {
        let pool = POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(3).build().expect("thumbnail pool"));
        pool.install(|| convertio_image::thumbnail(&path, 160))
    })
    .await
    .map_err(|e| e.to_string())?
    .map(tauri::ipc::Response::new)
    .map_err(|e| e.to_string())
}

/// Remember the options used for an operation, so Explorer right-click conversions
/// (which run without the window) use the same settings.
#[tauri::command]
pub fn remember_options(state: State<'_, AppState>, operation_id: String, options: OptionValues) -> Result<(), String> {
    crate::options::remember(&state.options_path, &operation_id, options).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn module_status(state: State<'_, AppState>, id: String) -> ModuleStatus {
    state.modules.status(&id, None)
}
