//! Tauri shell: wires the engine (`convertio-core` + converter crates) to the window.
//! Keep logic out of here — it belongs in the crates, where it is testable without a UI.

mod cli;
mod commands;
mod shell_integration;

use convertio_core::Registry;
use convertio_modules::ModuleManager;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

pub struct AppState {
    pub registry: Registry,
    pub modules: Arc<ModuleManager>,
    /// Files/operation passed on the command line at startup (Explorer context menu),
    /// picked up once by the UI when it is ready.
    pub pending_launch: Mutex<Option<cli::LaunchRequest>>,
}

fn build_registry() -> Registry {
    let mut r = Registry::new();
    r.register(convertio_image::ImageConverter)
        .register(convertio_audio::AudioConverter)
        .register(convertio_pdf::PdfConverter);
    r
}

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch (e.g. context menu on 10 files) forwards
        // its arguments to the running window instead of opening new ones.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            if let Some(req) = cli::parse(argv.into_iter().skip(1), Some(cwd.as_ref())) {
                let _ = app.emit("launch-request", req);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            app.manage(AppState {
                registry: build_registry(),
                modules: Arc::new(ModuleManager::new(&data_dir)),
                pending_launch: Mutex::new(cli::parse(std::env::args().skip(1), None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::take_launch_request,
            commands::inspect_files,
            commands::run_operation,
            commands::module_status,
            commands::set_shell_integration,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ConvertIO");
}
