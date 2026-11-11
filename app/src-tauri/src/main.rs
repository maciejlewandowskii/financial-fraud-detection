#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::path::PathBuf;

use commands::AppState;
use fraud_predict::Predictor;

fn model_dir() -> PathBuf {
    std::env::var_os("FRAUD_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts"))
}

fn main() {
    let predictor = Predictor::load(&model_dir()).map_err(|e| {
        format!("Brak wytrenowanego modelu w {} ({e}). Uruchom: cargo run -p fraud-train --release", model_dir().display())
    });
    tauri::Builder::default()
        .manage(AppState { predictor, dataset: Default::default() })
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
