use serde::Serialize;

use crate::Transaction;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Prediction {
    pub is_fraud: bool,
    pub fraud_probability: f32,
}

pub trait FraudModel: Send + Sync {
    fn name(&self) -> &str;
    fn predict(&self, transaction: &Transaction) -> Result<Prediction, Error>;
}
