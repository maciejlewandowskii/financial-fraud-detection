mod data;
mod error;
mod metrics;
mod training;

pub use error::{Error, Result};
pub use data::{Data, prepare};
pub use training::{TrainConfig, train};
