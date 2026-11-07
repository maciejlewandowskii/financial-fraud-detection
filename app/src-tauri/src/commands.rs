use std::sync::{Arc, Mutex};

use fraud_core::{Dataset, DatasetInfo, FraudModel, Page, Prediction, RandomRow, Transaction};
use serde::Serialize;
use tauri::State;

pub struct AppState {
    pub model: Box<dyn FraudModel>,
    pub dataset: Mutex<Option<Arc<Dataset>>>,
}

impl AppState {
    fn dataset(&self) -> Result<Arc<Dataset>, String> {
        self.dataset
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "Najpierw wybierz plik datasetu".to_string())
    }
}

#[derive(Serialize)]
pub struct ModelInfo {
    name: String,
}

#[tauri::command]
pub fn model_info(state: State<'_, AppState>) -> ModelInfo {
    ModelInfo {
        name: state.model.name().to_string(),
    }
}

#[tauri::command]
pub async fn predict(state: State<'_, AppState>, transaction: Transaction) -> Result<Prediction, String> {
    state.model.predict(&transaction).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_dataset(state: State<'_, AppState>, path: String) -> Result<DatasetInfo, String> {
    let dataset = tauri::async_runtime::spawn_blocking({
        let path = path.clone();
        move || Dataset::open(&path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Nie udało się otworzyć {path}: {e}"))?;
    let info = dataset.info();
    *state.dataset.lock().unwrap() = Some(Arc::new(dataset));
    Ok(info)
}

#[tauri::command]
pub async fn list_transactions(
    state: State<'_, AppState>,
    offset: usize,
    limit: usize,
    only_fraud: bool,
) -> Result<Page, String> {
    state.dataset()?.page(offset, limit, only_fraud).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn random_transaction(state: State<'_, AppState>, only_fraud: bool) -> Result<RandomRow, String> {
    state.dataset()?.random(only_fraud).map_err(|e| e.to_string())
}
