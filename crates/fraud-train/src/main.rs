use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use fraud_train::{TrainConfig, prepare, train};

#[derive(Parser)]
struct Args {
    /* Directory with train_transaction.csv (and optionally train_identity.csv). */
    #[arg(long, default_value = "data/ieee")]
    data: PathBuf,
    /* Where the trained model is written. */
    #[arg(long, default_value = "artifacts")]
    out: PathBuf,
    #[arg(long, default_value_t = TrainConfig::default().epochs)]
    epochs: usize,
    /* Only read the first N transactions (quick runs). */
    #[arg(long)]
    limit: Option<usize>,
    /* How many missed frauds one false alarm is worth. */
    #[arg(long, default_value_t = TrainConfig::default().fp_cost)]
    fp_cost: f64,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> fraud_train::Result<()> {
    let config = TrainConfig { epochs: args.epochs, fp_cost: args.fp_cost, ..TrainConfig::default() };

    let start = Instant::now();
    eprintln!("reading {}", args.data.display());
    let data = prepare(&args.data, args.limit, config.seed)?;
    eprintln!(
        "{} train / {} valid / {} test rows, {} numeric + {} categorical features, {} customers ({:.0?})",
        data.train.len(),
        data.valid.len(),
        data.test.len(),
        data.preprocessor.numeric_width(),
        data.preprocessor.cardinalities().len(),
        data.preprocessor.customers(),
        start.elapsed(),
    );

    let report = train(&data, &config, &args.out)?;

    let test = &report.test;
    println!("trained in {:.0?}, threshold {:.3}", start.elapsed(), report.threshold);
    println!(
        "test: AUC-PR {:.2}% (fraud rate {:.2}%), AUROC {:.2}%",
        100.0 * test.auc_pr,
        100.0 * test.frauds as f64 / test.rows as f64,
        100.0 * test.auroc
    );
    println!(
        "test at threshold: precision {:.1}%, recall {:.1}% ({} frauds caught, {} false alarms)",
        100.0 * test.precision,
        100.0 * test.recall,
        test.true_positives,
        test.false_positives
    );
    println!("saved to {}", args.out.display());
    Ok(())
}
