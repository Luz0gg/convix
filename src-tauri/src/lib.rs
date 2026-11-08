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
mod store;
mod sync;

use convix_core::{CancelToken, Registry};
use convix_modules::ModuleManager;
use settings::Settings;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};
use tauri::{AppHandle, Emitter, Manager, RunEvent};

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

impl AppState {
    /// A job runner with the current settings.
    pub fn runner(&self) -> jobs::Runner {
        jobs::Runner {
            registry: self.registry.clone(),
            modules: self.modules.clone(),
            settings: sync::read(&self.settings).clone(),
        }
    }
}

fn build_registry() -> Registry {
    let mut r = Registry::new();
    r.register(convix_image::ImageConverter);
    r.register(convix_audio::AudioConverter);
    r.register(convix_audio::VideoConverter);
    r.register(convix_pdf::PdfConverter);
    // Only converters that fully work are registered: the UI never offers an action that can't run.
    r
}

/// One writer at a time for the Explorer menu: it is rebuilt key by key, and two rebuilds
/// running together (startup + a settings change) would leave it half written.
static SHELL_MENU: Mutex<()> = Mutex::new(());

/// Make the Explorer menu match the settings (installed, removed, language, modules…).
/// Blocking (registry writes): call it off the UI thread.
/// Not in dev builds: they would register target\debug\convix.exe.
pub(crate) fn sync_shell_menu(
    registry: &Registry,
    modules: &ModuleManager,
    settings: &Settings,
) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Ok(());
    }
    let _one_at_a_time = sync::lock(&SHELL_MENU);
    if settings.shell_integration {
        shell_integration::install(registry, &settings.language, &|id| {
            modules.locate(id).is_some()
        })
    } else {
        shell_integration::uninstall()
    }
}

/// Rewrite the Explorer menu in the background (exe path, language, installed modules may have changed).
pub(crate) fn refresh_shell_menu(state: &AppState) {
    let settings = sync::read(&state.settings).clone();
    let (registry, modules) = (state.registry.clone(), state.modules.clone());
    std::thread::spawn(move || {
        if let Err(e) = sync_shell_menu(&registry, &modules, &settings) {
            log::error(&format!("Explorer menu | {e}"));
        }
    });
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
        sync::lock(&EARLY_LAUNCHES).push((argv, cwd));
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
    // First of all, so that even a crash while Tauri starts leaves a trace.
    if let Some(dir) = log::default_dir() {
        log::init(&dir);
    }
    let from_explorer = std::env::args_os().len() > 1;
    log::info(&format!(
        "Convix {} started{}",
        env!("CARGO_PKG_VERSION"),
        if from_explorer { " (Explorer menu)" } else { "" }
    ));

    let app = tauri::Builder::default()
        // Must be first.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            on_second_instance(app, argv, cwd)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_local_data_dir()?;
            log::init(&data_dir.join("logs")); // no-op unless LOCALAPPDATA was missing
            let settings_path = config_dir.join("settings.json");
            let settings = settings::load(&settings_path);
            let registry = build_registry();

            // Right-click › "Convertir en PNG": convert in the background, no window.
            let launch = cli::parse(std::env::args_os().skip(1), None);
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
                modules: Arc::new(ModuleManager::new(&data_dir)),
                settings: RwLock::new(settings),
                settings_path,
                options_path: config_dir.join("options.json"),
                batches: Mutex::new(HashMap::new()),
                pending_launch: Mutex::new(pending_launch),
                quiet: Mutex::new(quiet_state),
            });
            // The exe path changes after an update: keep the Explorer menu pointing at us.
            refresh_shell_menu(&app.state::<AppState>());
            let early = std::mem::take(&mut *sync::lock(&EARLY_LAUNCHES));
            for (argv, cwd) in early {
                on_second_instance(app.handle(), argv, cwd);
            }

            // The window is created hidden (tauri.conf.json) so quiet mode never flashes it.
            if is_quiet {
                let handle = app.handle().clone();
                std::thread::Builder::new()
                    .name("quiet".into())
                    .spawn(move || quiet::run(handle))?;
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
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(e) => {
            log::error(&format!("startup failed | {e}"));
            std::process::exit(1);
        }
    };
    app.run(|app, event| match event {
        RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { .. },
            ..
        } if label == "main" => {
            // Closing the window stops running conversions: say so in the log.
            if let Some(state) = app.try_state::<AppState>() {
                let running = sync::lock(&state.batches).len();
                if running > 0 {
                    log::info(&format!("window closed with {running} batch(es) running"));
                }
            }
        }
        RunEvent::Exit => log::info("exit"),
        _ => {}
    });
}
