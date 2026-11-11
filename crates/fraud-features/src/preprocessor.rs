use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::aggregates::{self, Aggregates};
use crate::table::{Row, TIME, Table, parse_number};
use crate::{Error, Result};

const UNKNOWN: u32 = 0;
const MISSING: u32 = 1;
const MIN_CATEGORY_COUNT: usize = 10;
const TIME_FEATURES: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preprocessor {
    numeric_columns: Vec<String>,
    categorical_columns: Vec<String>,
    amount: usize,
    aggregates: Aggregates,
    scaling: Vec<(f32, f32)>,
    missing_flags: Vec<usize>,
    vocabularies: Vec<HashMap<String, u32>>,
}

#[derive(Debug, Clone, Default)]
pub struct Features {
    pub width: usize,
    pub numeric: Vec<f32>,
    pub categorical: Vec<u32>,
    pub labels: Vec<bool>,
}

impl Features {
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn categorical_width(&self) -> usize {
        self.categorical.len().checked_div(self.len()).unwrap_or(0)
    }
}

impl Preprocessor {
    pub fn fit(table: &Table, train: &[usize]) -> Result<Self> {
        let aggregates =
            Aggregates::build((0..table.len()).map(|i| table.row(i)), &table.numeric_columns, &table.categorical_columns)?;
        let amount = table.numeric_columns.iter().position(|c| c == "TransactionAmt").ok_or(Error::MissingColumn("TransactionAmt"))?;
        let mut pre = Preprocessor {
            numeric_columns: table.numeric_columns.clone(),
            categorical_columns: table.categorical_columns.clone(),
            amount,
            aggregates,
            scaling: Vec::new(),
            missing_flags: Vec::new(),
            vocabularies: Vec::new(),
        };

        let values: Vec<Vec<f32>> = train.iter().map(|&i| pre.values(&table.row(i))).collect();
        let width = pre.numeric_columns.len() + aggregates::NAMES.len();
        pre.scaling = (0..width).map(|j| mean_std(values.iter().map(|v| v[j]))).collect();
        pre.missing_flags = distinct_missing_patterns(&values, width);

        let mut counts: Vec<HashMap<&str, usize>> = vec![HashMap::new(); pre.categorical_columns.len()];
        for &i in train {
            for (count, value) in counts.iter_mut().zip(table.row(i).categorical) {
                *count.entry(value).or_default() += 1;
            }
        }
        pre.vocabularies = counts.into_iter().map(vocabulary).collect();
        Ok(pre)
    }

    pub fn numeric_width(&self) -> usize {
        self.scaling.len() + TIME_FEATURES + self.missing_flags.len()
    }

    pub fn cardinalities(&self) -> Vec<usize> {
        self.vocabularies.iter().map(|v| v.len() + 2).collect()
    }

    pub fn customers(&self) -> usize {
        self.aggregates.customers()
    }

    pub fn encode(&self, table: &Table, rows: &[usize]) -> Features {
        let mut features = Features { width: self.numeric_width(), ..Default::default() };
        for &i in rows {
            self.encode_row(&table.row(i), &mut features.numeric, &mut features.categorical);
            features.labels.push(table.labels[i]);
        }
        features
    }

    pub fn transform(&self, record: &HashMap<String, String>) -> (Vec<f32>, Vec<u32>) {
        let text = |column: &String| record.get(column).map_or("", |v| v.trim());
        let numeric: Vec<f32> = self.numeric_columns.iter().map(|c| parse_number(text(c))).collect();
        let row = Row {
            time: record.get(TIME).and_then(|v| v.trim().parse().ok()).unwrap_or(0),
            numeric: &numeric,
            categorical: self.categorical_columns.iter().map(text).collect(),
        };
        let (mut numeric, mut categorical) = (Vec::new(), Vec::new());
        self.encode_row(&row, &mut numeric, &mut categorical);
        (numeric, categorical)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        Ok(std::fs::write(path, serde_json::to_string(self)?)?)
    }

    pub fn load(path: &Path) -> Result<Self> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    fn values(&self, row: &Row) -> Vec<f32> {
        let mut values = row.numeric.to_vec();
        values.extend(self.aggregates.features(row));
        values
    }

    fn encode_row(&self, row: &Row, numeric: &mut Vec<f32>, categorical: &mut Vec<u32>) {
        let values = self.values(row);
        numeric.extend(values.iter().zip(&self.scaling).map(|(&x, &(mean, std))| {
            if x.is_nan() { 0.0 } else { (signed_log1p(x) - mean) / std }
        }));
        numeric.extend(time_features(values[self.amount], row.time));
        numeric.extend(self.missing_flags.iter().map(|&j| values[j].is_nan() as u8 as f32));
        categorical.extend(row.categorical.iter().zip(&self.vocabularies).map(|(value, vocabulary)| match *value {
            "" => MISSING,
            value => vocabulary.get(value).copied().unwrap_or(UNKNOWN),
        }));
    }
}

fn signed_log1p(x: f32) -> f32 {
    x.signum() * x.abs().ln_1p()
}

fn time_features(amount: f32, time: i64) -> [f32; TIME_FEATURES] {
    let cents = if amount.is_nan() { 0.0 } else { amount.fract() };
    let hour = TAU * time.rem_euclid(86_400) as f32 / 86_400.0;
    let weekday = TAU * time.div_euclid(86_400).rem_euclid(7) as f32 / 7.0;
    [cents, hour.sin(), hour.cos(), weekday.sin(), weekday.cos()]
}

fn mean_std(values: impl Iterator<Item = f32>) -> (f32, f32) {
    let (mut n, mut sum, mut sum_sq) = (0.0, 0.0, 0.0);
    for x in values.filter(|x| !x.is_nan()).map(|x| signed_log1p(x) as f64) {
        n += 1.0;
        sum += x;
        sum_sq += x * x;
    }
    if n == 0.0 {
        return (0.0, 1.0);
    }
    let mean = sum / n;
    let std = (sum_sq / n - mean * mean).max(0.0).sqrt();
    (mean as f32, if std > 1e-6 { std as f32 } else { 1.0 })
}

/* Columns go missing in blocks (e.g. V1–V11 together), so one flag per pattern is enough. */
fn distinct_missing_patterns(values: &[Vec<f32>], width: usize) -> Vec<usize> {
    let mut seen = std::collections::HashSet::new();
    (0..width)
        .filter(|&j| {
            let pattern: Vec<bool> = values.iter().map(|v| v[j].is_nan()).collect();
            let sometimes_missing = pattern.contains(&true) && pattern.contains(&false);
            sometimes_missing && seen.insert(pattern)
        })
        .collect()
}

fn vocabulary(counts: HashMap<&str, usize>) -> HashMap<String, u32> {
    let mut kept: Vec<(&str, usize)> =
        counts.into_iter().filter(|&(value, n)| !value.is_empty() && n >= MIN_CATEGORY_COUNT).collect();
    kept.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    kept.into_iter().enumerate().map(|(rank, (value, _))| (value.to_owned(), rank as u32 + 2)).collect()
}
