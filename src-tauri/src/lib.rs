//! Tauri shell: wires the engine (`convertio-core` + converter crates) to the window.
//! Keep logic out of here — it belongs in the crates, where it is testable without a UI.

mod cli;
mod commands;
mod jobs;
mod settings;
mod shell_integration;

use convertio_core::{CancelToken, Registry};
use convertio_modules::ModuleManager;
use settings::Settings;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};
use tauri::{Emitter, Manager};

pub struct AppState {
    pub registry: Registry,
    pub modules: Arc<ModuleManager>,
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    /// Running batches, by id, so the UI can cancel them.
    pub batches: Mutex<HashMap<String, CancelToken>>,
    /// Files/operation passed on the command line at startup (Explorer context menu),
    /// picked up once by the UI when it is ready.
    pub pending_launch: Mutex<Option<cli::LaunchRequest>>,
}

fn build_registry() -> Registry {
    let mut r = Registry::new();
    r.register(convertio_image::ImageConverter);
    // Registered once implemented, so the UI never offers actions that can't run yet:
    // Phase 2: r.register(convertio_audio::AudioConverter);
    // Phase 3: r.register(convertio_pdf::PdfConverter);
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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = settings::load(&settings_path);
            let registry = build_registry();

            // Refresh the Explorer menu (the exe path changes after an update).
            // Not in dev builds: they would register target\debug\convertio.exe.
            if settings.shell_integration && !cfg!(debug_assertions) {
                let (registry, lang) = (registry.clone(), settings.language.clone());
                std::thread::spawn(move || shell_integration::install(&registry, &lang));
            }

            app.manage(AppState {
                registry,
                modules: Arc::new(ModuleManager::new(&app.path().app_local_data_dir()?)),
                settings: RwLock::new(settings),
                settings_path,
                batches: Mutex::new(HashMap::new()),
                pending_launch: Mutex::new(cli::parse(std::env::args().skip(1), None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::take_launch_request,
            commands::get_settings,
            commands::update_settings,
            commands::inspect_files,
            commands::run_batch,
            commands::cancel_batch,
            commands::module_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ConvertIO");
}
