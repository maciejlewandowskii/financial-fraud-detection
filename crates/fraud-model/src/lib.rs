use std::path::Path;

use burn::nn::{Dropout, DropoutConfig, Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::*;
use burn::store::{BurnpackStore, ModuleSnapshot};
use burn::tensor::activation::{relu, sigmoid, softplus};
use burn::train::{ClassificationOutput, InferenceStep, TrainOutput, TrainStep};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("saving model: {0}")]
    Save(String),
    #[error("loading model: {0}")]
    Load(String),
    #[error("weights do not match the model: {0}")]
    Mismatch(String),
}

#[derive(Config, Debug)]
pub struct ModelConfig {
    pub numeric_width: usize,
    pub cardinalities: Vec<usize>,
    #[config(default = 8)]
    pub embedding_dim: usize,
    #[config(default = "vec![512, 256, 128]")]
    pub hidden: Vec<usize>,
    #[config(default = 0.4)]
    pub dropout: f64,
    #[config(default = 0.1)]
    pub input_dropout: f64,
}

#[derive(Module, Debug)]
pub struct FraudModel {
    embedding: Embedding,
    #[module(skip)]
    offsets: Vec<i64>,
    hidden: Vec<Linear>,
    output: Linear,
    input_dropout: Dropout,
    dropout: Dropout,
}

impl ModelConfig {
    pub fn init(&self, device: &Device) -> FraudModel {
        let offsets = self
            .cardinalities
            .iter()
            .scan(0, |start, &n| {
                let offset = *start;
                *start += n as i64;
                Some(offset)
            })
            .collect();
        let mut width = self.numeric_width + self.cardinalities.len() * self.embedding_dim;
        let hidden = self
            .hidden
            .iter()
            .map(|&size| {
                let layer = LinearConfig::new(width, size).init(device);
                width = size;
                layer
            })
            .collect();
        FraudModel {
            embedding: EmbeddingConfig::new(self.cardinalities.iter().sum(), self.embedding_dim).init(device),
            offsets,
            hidden,
            output: LinearConfig::new(width, 1).init(device),
            input_dropout: DropoutConfig::new(self.input_dropout).init(),
            dropout: DropoutConfig::new(self.dropout).init(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Batch {
    pub numeric: Tensor<2>,
    pub categorical: Tensor<2, Int>,
}

impl Batch {
    pub fn new(numeric: &[f32], categorical: &[u32], rows: usize, device: &Device) -> Self {
        let categorical: Vec<i64> = categorical.iter().map(|&c| c as i64).collect();
        Self {
            numeric: Tensor::from_data(TensorData::new(numeric.to_vec(), [rows, numeric.len() / rows]), device),
            categorical: Tensor::from_data(TensorData::new(categorical.clone(), [rows, categorical.len() / rows]), device),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LabeledBatch {
    pub batch: Batch,
    pub targets: Tensor<1, Int>,
}

impl FraudModel {
    pub fn forward(&self, batch: Batch) -> Tensor<1> {
        let [rows, columns] = batch.categorical.dims();
        let offsets = Tensor::<1, Int>::from_data(TensorData::new(self.offsets.clone(), [columns]), &batch.categorical.device());
        let embedded = self.embedding.forward(batch.categorical + offsets.unsqueeze()).reshape([rows as i32, -1]);

        let mut x = self.input_dropout.forward(Tensor::cat(vec![batch.numeric, embedded], 1));
        for layer in &self.hidden {
            x = self.dropout.forward(relu(layer.forward(x)));
        }
        self.output.forward(x).reshape([rows])
    }

    pub fn predict(&self, batch: Batch) -> Tensor<1> {
        sigmoid(self.forward(batch))
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        let mut store = BurnpackStore::from_file(path).overwrite(true);
        self.save_into(&mut store).map_err(|e| Error::Save(e.to_string()))
    }

    pub fn load(config: &ModelConfig, path: &Path, device: &Device) -> Result<Self, Error> {
        let mut model = config.init(device);
        let mut store = BurnpackStore::from_file(path);
        let result = model.load_from(&mut store).map_err(|e| Error::Load(e.to_string()))?;
        if !result.is_success() || !result.missing.is_empty() {
            return Err(Error::Mismatch(format!("{result:?}")));
        }
        Ok(model)
    }
}

pub fn bce_loss(logits: Tensor<1>, targets: Tensor<1>) -> Tensor<1> {
    let positive = targets.clone() * softplus(logits.clone().neg(), 1.0);
    let negative = (targets.neg() + 1.0) * softplus(logits, 1.0);
    (positive + negative).mean()
}

impl FraudModel {
    fn classify(&self, input: LabeledBatch) -> ClassificationOutput {
        let logits = self.forward(input.batch);
        let loss = bce_loss(logits.clone(), input.targets.clone().float());
        ClassificationOutput::new(loss, sigmoid(logits).unsqueeze_dim(1), input.targets)
    }
}

impl TrainStep for FraudModel {
    type Input = LabeledBatch;
    type Output = ClassificationOutput;

    fn step(&self, input: LabeledBatch) -> TrainOutput<ClassificationOutput> {
        let output = self.classify(input);
        TrainOutput::new(self, output.loss.backward(), output)
    }
}

impl InferenceStep for FraudModel {
    type Input = LabeledBatch;
    type Output = ClassificationOutput;

    fn step(&self, input: LabeledBatch) -> ClassificationOutput {
        self.classify(input)
    }
}
