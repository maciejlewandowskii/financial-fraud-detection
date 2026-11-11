use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}: {source}")]
    Read { path: PathBuf, source: csv::Error },
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("missing column {0}")]
    MissingColumn(&'static str),
    #[error("invalid {column} value {value:?}")]
    InvalidValue { column: &'static str, value: String },
    #[error("no transactions read")]
    NoRows,
}

pub type Result<T> = std::result::Result<T, Error>;
