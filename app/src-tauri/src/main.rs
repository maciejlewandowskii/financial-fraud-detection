#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Mutex;

use commands::AppState;
use fraud_core::HeuristicModel;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            model: Box::new(HeuristicModel),
            dataset: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::model_info,
            commands::predict,
            commands::open_dataset,
            commands::list_transactions,
            commands::random_transaction,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
