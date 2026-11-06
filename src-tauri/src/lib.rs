//! Tauri shell: wires the engine (`convix-core` + converter crates) to the window.
//! Keep logic out of here — it belongs in the crates, where it is testable without a UI.

mod cli;
mod commands;
mod jobs;
mod log;
mod options;
mod quiet;
mod settings;
mod shell_integration;

use convix_core::{CancelToken, Registry};
use convix_modules::ModuleManager;
use settings::Settings;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};
use tauri::{AppHandle, Emitter, Manager};

pub struct AppState {
    pub registry: Registry,
    pub modules: Arc<ModuleManager>,
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    /// Last options used per operation (shared by the UI and quiet conversions).
    pub options_path: PathBuf,
    /// Running batches, by id, so the UI can cancel them.
    pub batches: Mutex<HashMap<String, CancelToken>>,
    /// Files/operation passed on the command line at startup (Explorer context menu),
    /// picked up once by the UI when it is ready.
    pub pending_launch: Mutex<Option<cli::LaunchRequest>>,
    /// Background conversions started from the Explorer menu (no window).
    pub quiet: Mutex<quiet::QuietState>,
}

fn build_registry() -> Registry {
    let mut r = Registry::new();
    r.register(convix_image::ImageConverter);
    r.register(convix_audio::AudioConverter);
    r.register(convix_pdf::PdfConverter);
    // Only converters that fully work are registered: the UI never offers an action that can't run.
    r
}

/// Rewrite the Explorer menu (exe path, language, installed modules may have changed).
/// Not in dev builds: they would register target\debug\convix.exe.
pub(crate) fn refresh_shell_menu(state: &AppState) {
    let settings = state.settings.read().unwrap().clone();
    if settings.shell_integration && !cfg!(debug_assertions) {
        let (registry, modules) = (state.registry.clone(), state.modules.clone());
        std::thread::spawn(move || {
            shell_integration::install(&registry, &settings.language, &|id| {
                modules.locate(id).is_some()
            })
        });
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Launches forwarded while we are still starting up (state not managed yet).
/// Selecting many files in Explorer starts them all at once, so this does happen.
static EARLY_LAUNCHES: Mutex<Vec<(Vec<String>, String)>> = Mutex::new(Vec::new());

/// A second launch (e.g. context menu on 10 files) lands here instead of opening a new app.
fn on_second_instance(app: &AppHandle, argv: Vec<String>, cwd: String) {
    let Some(state) = app.try_state::<AppState>() else {
        EARLY_LAUNCHES.lock().unwrap().push((argv, cwd));
        return;
    };
    if let Some(req) = cli::parse(argv.into_iter().skip(1), Some(cwd.as_ref())) {
        if quiet::enqueue(&state, req.clone()) {
            return; // handled in the background, window stays hidden
        }
        let _ = app.emit("launch-request", req);
    }
    quiet::keep_open(&state);
    show_main_window(app);
}

pub fn run() {
    tauri::Builder::default()
        // Must be first.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            on_second_instance(app, argv, cwd)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            log::init(&app.path().app_local_data_dir()?.join("logs"));
            log::info(&format!("Convix {} started{}", env!("CARGO_PKG_VERSION"), if std::env::args().len() > 1 { " (Explorer menu)" } else { "" }));
            let settings_path = config_dir.join("settings.json");
            let settings = settings::load(&settings_path);
            let registry = build_registry();

            // Right-click › "Convertir en PNG": convert in the background, no window.
            let launch = cli::parse(std::env::args().skip(1), None);
            let is_quiet = settings.quiet_context_menu
                && launch.as_ref().is_some_and(|r| r.operation.is_some());
            let mut quiet_state = quiet::QuietState {
                active: is_quiet,
                ..Default::default()
            };
            let pending_launch = if is_quiet {
                quiet_state.pending.extend(launch);
                None
            } else {
                launch
            };

            app.manage(AppState {
                registry,
                modules: Arc::new(ModuleManager::new(&app.path().app_local_data_dir()?)),
                settings: RwLock::new(settings),
                settings_path,
                options_path: config_dir.join("options.json"),
                batches: Mutex::new(HashMap::new()),
                pending_launch: Mutex::new(pending_launch),
                quiet: Mutex::new(quiet_state),
            });
            // The exe path changes after an update: keep the Explorer menu pointing at us.
            refresh_shell_menu(&app.state::<AppState>());
            let early = std::mem::take(&mut *EARLY_LAUNCHES.lock().unwrap());
            for (argv, cwd) in early {
                on_second_instance(app.handle(), argv, cwd);
            }

            // The window is created hidden (tauri.conf.json) so quiet mode never flashes it.
            if is_quiet {
                let handle = app.handle().clone();
                std::thread::spawn(move || quiet::run(handle));
            } else {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::take_launch_request,
            commands::get_settings,
            commands::update_settings,
            commands::inspect_files,
            commands::thumbnail,
            commands::run_batch,
            commands::cancel_batch,
            commands::remember_options,
            commands::log_ui,
            commands::module_list,
            commands::install_module,
            commands::remove_module,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Convix");
}
