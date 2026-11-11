use std::sync::{Arc, Mutex};

use fraud_browser::{Dataset, DatasetInfo, Page, RandomRow, Transaction};
use fraud_predict::{Prediction, Predictor};
use serde::Serialize;
use tauri::State;

pub struct AppState {
    pub predictor: Result<Predictor, String>,
    pub dataset: Mutex<Option<Arc<Dataset>>>,
}

impl AppState {
    fn predictor(&self) -> Result<&Predictor, String> {
        self.predictor.as_ref().map_err(Clone::clone)
    }

    fn dataset(&self) -> Result<Arc<Dataset>, String> {
        self.dataset.lock().unwrap().clone().ok_or_else(|| "Najpierw wybierz plik datasetu".to_string())
    }
}

#[derive(Serialize)]
pub struct ModelInfo {
    name: String,
}

#[tauri::command]
pub fn model_info(state: State<'_, AppState>) -> ModelInfo {
    let name = match state.predictor() {
        Ok(predictor) => {
            let report = predictor.report();
            format!(
                "sieć neuronowa (MLP) · AUC-PR na teście {:.1}% · próg {:.3}",
                100.0 * report.test.auc_pr,
                report.threshold
            )
        }
        Err(_) => "brak wytrenowanego modelu".to_string(),
    };
    ModelInfo { name }
}

#[tauri::command]
pub async fn predict(state: State<'_, AppState>, transaction: Transaction) -> Result<Prediction, String> {
    Ok(state.predictor()?.predict(&transaction))
}

#[tauri::command]
pub async fn open_dataset(state: State<'_, AppState>) -> Result<DatasetInfo, String> {
    let dir = state.predictor()?.report().data.clone();
    let dataset = tauri::async_runtime::spawn_blocking({
        let dir = dir.clone();
        move || Dataset::open(dir)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("Nie udało się otworzyć danych treningowych z {}: {e}", dir.display()))?;
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
