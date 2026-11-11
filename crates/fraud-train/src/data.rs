use std::path::{Path, PathBuf};
use std::sync::Arc;

use burn::data::dataloader::batcher::Batcher;
use burn::data::dataloader::{DataLoader, DataLoaderBuilder};
use burn::data::dataset::{Dataset, DatasetError};
use burn::prelude::*;
use fraud_features::{Features, Preprocessor, Table};
use fraud_model::{Batch, FraudModel, LabeledBatch};

const TRAIN_FRACTION: f64 = 0.8;
const VALID_FRACTION: f64 = 0.1;

pub struct Data {
    pub dir: PathBuf,
    pub preprocessor: Preprocessor,
    pub train: Arc<Features>,
    pub valid: Arc<Features>,
    pub test: Arc<Features>,
}

pub fn prepare(dir: &Path, limit: Option<usize>, seed: u64) -> crate::Result<Data> {
    let table = Table::read(dir, limit)?;
    let [train, valid, test] = split(&table.labels, seed);
    let preprocessor = Preprocessor::fit(&table, &train)?;
    Ok(Data {
        dir: dir.canonicalize()?,
        train: Arc::new(preprocessor.encode(&table, &train)),
        valid: Arc::new(preprocessor.encode(&table, &valid)),
        test: Arc::new(preprocessor.encode(&table, &test)),
        preprocessor,
    })
}

/* Stratified, so every part has the same fraud rate. */
fn split(labels: &[bool], seed: u64) -> [Vec<usize>; 3] {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut parts: [Vec<usize>; 3] = Default::default();
    for class in [false, true] {
        let mut rows: Vec<usize> = (0..labels.len()).filter(|&i| labels[i] == class).collect();
        rng.shuffle(&mut rows);
        let train_end = (rows.len() as f64 * TRAIN_FRACTION).round() as usize;
        let valid_end = (train_end + (rows.len() as f64 * VALID_FRACTION).round() as usize).min(rows.len());
        parts[0].extend(&rows[..train_end]);
        parts[1].extend(&rows[train_end..valid_end]);
        parts[2].extend(&rows[valid_end..]);
    }
    parts.each_mut().map(|part| {
        part.sort_unstable();
        std::mem::take(part)
    })
}

#[derive(Debug, Clone)]
pub struct Item {
    numeric: Vec<f32>,
    categorical: Vec<u32>,
    label: bool,
}

struct Rows(Arc<Features>);

impl Dataset<Item> for Rows {
    fn get(&self, i: usize) -> Result<Item, DatasetError> {
        let (f, width, categories) = (&self.0, self.0.width, self.0.categorical_width());
        Ok(Item {
            numeric: f.numeric[i * width..(i + 1) * width].to_vec(),
            categorical: f.categorical[i * categories..(i + 1) * categories].to_vec(),
            label: f.labels[i],
        })
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone)]
struct ItemBatcher;

impl Batcher<Item, LabeledBatch> for ItemBatcher {
    fn batch(&self, items: Vec<Item>, device: &Device) -> LabeledBatch {
        let numeric: Vec<f32> = items.iter().flat_map(|i| i.numeric.iter().copied()).collect();
        let categorical: Vec<u32> = items.iter().flat_map(|i| i.categorical.iter().copied()).collect();
        let labels: Vec<i64> = items.iter().map(|i| i.label as i64).collect();
        LabeledBatch {
            batch: Batch::new(&numeric, &categorical, items.len(), device),
            targets: Tensor::from_data(TensorData::new(labels, [items.len()]), device),
        }
    }
}

pub fn loader(features: &Arc<Features>, batch_size: usize, shuffle: Option<u64>, device: &Device) -> Arc<dyn DataLoader<LabeledBatch>> {
    let builder = DataLoaderBuilder::new(ItemBatcher).batch_size(batch_size).num_workers(4).set_device(device.clone());
    let builder = match shuffle {
        Some(seed) => builder.shuffle(seed),
        None => builder,
    };
    builder.build(Rows(features.clone()))
}

pub fn predict(model: &FraudModel, features: &Arc<Features>, device: &Device) -> crate::Result<Vec<f32>> {
    let rows = Rows(features.clone());
    let mut probs = Vec::with_capacity(features.len());
    for start in (0..features.len()).step_by(16_384) {
        let items = (start..(start + 16_384).min(features.len())).map(|i| rows.get(i)).collect::<Result<_, _>>()?;
        let data = model.predict(ItemBatcher.batch(items, device).batch).into_data();
        probs.extend(data.try_to_vec::<f32>()?);
    }
    Ok(probs)
}
