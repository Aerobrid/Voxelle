//! Voxelle core library: everything the desktop app runs, reusable from tools and tests.

pub mod app_state;
pub mod asr;
pub mod audio;
pub mod commands;
pub mod dictation;
pub mod hotkeys;
pub mod insert;
pub mod models;
pub mod net;
pub mod overlay;
pub mod session;
pub mod storage;
pub mod tray;

use tauri::Manager;

use app_state::AppState;

fn platform_inserter() -> Box<dyn insert::TextInserter> {
    #[cfg(windows)]
    return Box::new(insert::WindowsInserter::new());
    #[cfg(not(windows))]
    return Box::new(insert::UnsupportedInserter);
}

/// First run: with no model chosen, pick the recommended installed one for this build (P0 §0.2).
fn select_default_model(state: &AppState) {
    let dir = state.model_dir();
    let mut settings = state.settings.lock().unwrap();
    if settings.selected_speech_model_id.is_some() {
        return;
    }
    let installed: Vec<&str> = models::CATALOG
        .iter()
        .filter(|e| models::is_installed(&dir, e))
        .map(|e| e.id)
        .collect();
    let gpu = settings.prefer_gpu && asr::GPU_COMPILED;
    if let Some(id) = models::recommend(&installed, |id| models::reference_p50_ms(id, gpu)) {
        settings.selected_speech_model_id = Some(id.to_string());
        let _ = state.save_settings(&settings);
    }
}

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch hands over to the running instance and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .setup(|app| {
            let model_dir = app.path().app_local_data_dir()?.join("models");
            std::fs::create_dir_all(&model_dir)?;
            let db_path = app.path().app_data_dir().ok().map(|d| d.join("voxelle.db"));
            app.manage(AppState::new(model_dir, platform_inserter(), db_path));
            let state = app.state::<AppState>();
            select_default_model(&state);
            state.apply_retention();
            dictation::preload_selected_model(app.handle());
            let bindings = hotkeys::Bindings::from_settings(&state.settings.lock().unwrap())
                .unwrap_or_default();
            if let Err(e) = tray::create(app.handle()) {
                eprintln!("[voxelle] tray icon unavailable: {e}");
            }
            if let Err(e) = overlay::create(app.handle()) {
                eprintln!("[voxelle] recording indicator unavailable: {e}");
            }
            match hotkeys::start(app.handle(), bindings) {
                Ok(shared) => *state.hotkeys.lock().unwrap() = Some(shared),
                // Dictation still works from the window; the UI shows shortcuts as unavailable.
                Err(e) => eprintln!("[voxelle] global shortcuts unavailable: {e}"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_session_snapshot,
            commands::start_dictation,
            commands::stop_dictation,
            commands::cancel_dictation,
            commands::get_settings,
            commands::update_settings,
            commands::list_models,
            commands::select_model,
            commands::verify_model,
            commands::asr_capabilities,
            commands::get_history,
            commands::delete_history_entry,
            commands::clear_history,
            commands::list_input_devices,
            commands::copy_text,
            commands::set_history_edit,
            commands::get_model_dir,
            commands::open_model_folder,
            commands::export_history,
            commands::storage_info,
            commands::start_shortcut_capture,
            commands::cancel_shortcut_capture,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Voxelle");
}
