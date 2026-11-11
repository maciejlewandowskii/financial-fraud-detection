#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Features(#[from] fraud_features::Error),
    #[error(transparent)]
    Model(#[from] fraud_model::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Dataset(#[from] burn::data::dataset::DatasetError),
    #[error(transparent)]
    Data(#[from] burn::tensor::DataError),
    #[error("learning rate scheduler: {0}")]
    Scheduler(String),
}

pub type Result<T> = std::result::Result<T, Error>;
