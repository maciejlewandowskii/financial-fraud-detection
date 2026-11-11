use std::path::Path;

use burn::grad_clipping::GradientClippingConfig;
use burn::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig;
use burn::optim::AdamWConfig;
use burn::prelude::*;
use burn::train::metric::{AucPrMetric, AurocMetric, LossMetric, PrecisionMetric, RecallMetric};
use burn::train::{Learner, SupervisedTraining};
use fraud_model::{FraudModel, ModelConfig};
use fraud_predict::{Evaluation, MODEL_CONFIG_FILE, MODEL_FILE, PREPROCESSOR_FILE, REPORT_FILE, Report};

use crate::data::{Data, loader, predict};
use crate::{Error, Result};
use crate::metrics::{Confusion, auc_pr, auroc, cheapest_threshold};

#[derive(Debug, Clone)]
pub struct TrainConfig {
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub min_learning_rate: f64,
    pub weight_decay: f32,
    pub fp_cost: f64,
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            epochs: 40,
            batch_size: 4096,
            learning_rate: 1e-3,
            min_learning_rate: 1e-5,
            weight_decay: 1e-2,
            fp_cost: 10.0,
            seed: 42,
        }
    }
}

pub fn train(data: &Data, config: &TrainConfig, out: &Path) -> Result<Report> {
    let device = Device::flex();
    let train_device = device.clone().autodiff();
    train_device.seed(config.seed);

    let model_config = ModelConfig::new(data.preprocessor.numeric_width(), data.preprocessor.cardinalities());
    let optimizer = AdamWConfig::new()
        .with_weight_decay(config.weight_decay)
        .with_grad_clipping(Some(GradientClippingConfig::Norm(1.0)))
        .init();
    let total_steps = config.epochs * data.train.len().div_ceil(config.batch_size);
    let scheduler = CosineAnnealingLrSchedulerConfig::new(config.learning_rate, total_steps)
        .with_min_lr(config.min_learning_rate)
        .init()
        .map_err(Error::Scheduler)?;
    let learner = Learner::new(model_config.init(&train_device), optimizer, scheduler);

    let result = SupervisedTraining::new(
        out,
        loader(&data.train, config.batch_size, Some(config.seed), &train_device),
        loader(&data.valid, config.batch_size, None, &train_device),
    )
    .metrics((
        LossMetric::new(),
        AucPrMetric::binary(),
        AurocMetric::binary(),
        PrecisionMetric::binary(0.5),
        RecallMetric::binary(0.5),
    ))
    .num_epochs(config.epochs)
    .summary()
    .launch(learner);
    drop(result.renderer);
    let model = result.model;

    let valid_probs = predict(&model, &data.valid, &device)?;
    let threshold = cheapest_threshold(&valid_probs, &data.valid.labels, config.fp_cost, 1.0);
    let report = Report {
        data: data.dir.clone(),
        threshold,
        epochs: config.epochs,
        valid_auc_pr: auc_pr(&valid_probs, &data.valid.labels),
        test: evaluate(&model, data, threshold, &device)?,
    };

    std::fs::create_dir_all(out)?;
    model.save(&out.join(MODEL_FILE))?;
    model_config.save(out.join(MODEL_CONFIG_FILE))?;
    data.preprocessor.save(&out.join(PREPROCESSOR_FILE))?;
    std::fs::write(out.join(REPORT_FILE), serde_json::to_string_pretty(&report)?)?;
    Ok(report)
}

fn evaluate(model: &FraudModel, data: &Data, threshold: f32, device: &Device) -> Result<Evaluation> {
    let labels = &data.test.labels;
    let probs = predict(model, &data.test, device)?;
    let confusion = Confusion::at(&probs, labels, threshold);
    Ok(Evaluation {
        rows: labels.len(),
        frauds: labels.iter().filter(|&&y| y).count(),
        auc_pr: auc_pr(&probs, labels),
        auroc: auroc(&probs, labels),
        precision: confusion.precision(),
        recall: confusion.recall(),
        true_positives: confusion.true_positives,
        false_positives: confusion.false_positives,
    })
}
