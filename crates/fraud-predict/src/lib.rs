use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use burn::prelude::*;
use fraud_features::Preprocessor;
use fraud_model::{Batch, FraudModel, ModelConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reading model.json: {0}")]
    Config(String),
    #[error(transparent)]
    Model(#[from] fraud_model::Error),
    #[error(transparent)]
    Features(#[from] fraud_features::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub const MODEL_FILE: &str = "model.bpk";
pub const MODEL_CONFIG_FILE: &str = "model.json";
pub const PREPROCESSOR_FILE: &str = "preprocessor.json";
pub const REPORT_FILE: &str = "report.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub data: PathBuf,
    pub threshold: f32,
    pub epochs: usize,
    pub valid_auc_pr: f64,
    pub test: Evaluation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub rows: usize,
    pub frauds: usize,
    pub auc_pr: f64,
    pub auroc: f64,
    pub precision: f64,
    pub recall: f64,
    pub true_positives: usize,
    pub false_positives: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Prediction {
    pub is_fraud: bool,
    pub fraud_probability: f32,
}

pub struct Predictor {
    model: Mutex<FraudModel>,
    preprocessor: Preprocessor,
    report: Report,
    device: Device,
}

impl Predictor {
    pub fn load(dir: &Path) -> Result<Self, Error> {
        let device = Device::flex();
        let config = ModelConfig::load(dir.join(MODEL_CONFIG_FILE)).map_err(|e| Error::Config(e.to_string()))?;
        let model = FraudModel::load(&config, &dir.join(MODEL_FILE), &device)?;
        let report = std::fs::read_to_string(dir.join(REPORT_FILE))?;
        Ok(Self {
            model: Mutex::new(model),
            preprocessor: Preprocessor::load(&dir.join(PREPROCESSOR_FILE))?,
            report: serde_json::from_str(&report)?,
            device,
        })
    }

    pub fn report(&self) -> &Report {
        &self.report
    }

    pub fn predict(&self, transaction: &HashMap<String, String>) -> Prediction {
        let (numeric, categorical) = self.preprocessor.transform(transaction);
        let batch = Batch::new(&numeric, &categorical, 1, &self.device);
        let probability: f32 = self.model.lock().unwrap().predict(batch).into_scalar();
        Prediction { is_fraud: probability >= self.report.threshold, fraud_probability: probability }
    }
}
