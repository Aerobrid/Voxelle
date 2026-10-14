// Voxelle — local, private-first offline dictation & voice AI.
// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod asr;
mod audio;
mod commands;
mod insert;
mod models;
mod net;
mod session;
mod storage;

use app_state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_session_snapshot,
            commands::start_dictation,
            commands::stop_dictation,
            commands::cancel_dictation,
            commands::get_settings,
            commands::update_settings,
            commands::list_models,
            commands::select_model,
            commands::get_history,
            commands::delete_history_entry,
            commands::clear_history,
            commands::list_input_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Voxelle");
}
