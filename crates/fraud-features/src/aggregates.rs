use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::table::Row;
use crate::{Error, Result};

pub const NAMES: [&str; 17] = [
    "uid_count",
    "uid_amt_mean",
    "uid_amt_std",
    "uid_amt_ratio",
    "uid_C1_mean",
    "uid_C13_mean",
    "uid_C14_mean",
    "uid_D15_mean",
    "uid_dist1_mean",
    "uid_P_emaildomain_nunique",
    "uid_R_emaildomain_nunique",
    "uid_day_span",
    "D1n",
    "card1_count",
    "card2_count",
    "addr1_count",
    "P_emaildomain_count",
];

const AMOUNT: usize = 0;
const D1: usize = 1;
const NUMBERS: [&str; 7] = ["TransactionAmt", "D1", "C1", "C13", "C14", "D15", "dist1"];

const CARD1: usize = 0;
const ADDR1: usize = 2;
const P_EMAIL: usize = 3;
const R_EMAIL: usize = 4;
const TEXTS: [&str; 5] = ["card1", "card2", "addr1", "P_emaildomain", "R_emaildomain"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregates {
    numbers: [usize; 7],
    texts: [usize; 5],
    customers: HashMap<String, Customer>,
    counts: [HashMap<String, u32>; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Customer {
    count: u32,
    amount_mean: Option<f32>,
    amount_std: Option<f32>,
    means: [Option<f32>; 5],
    p_emails: u32,
    r_emails: u32,
    day_span: i64,
}

struct Keys<'a> {
    numbers: [f32; 7],
    texts: [&'a str; 5],
    day: i64,
}

impl Keys<'_> {
    fn start_day(&self) -> Option<i64> {
        let d1 = self.numbers[D1];
        (!d1.is_nan()).then(|| self.day - d1.round() as i64)
    }

    fn customer(&self) -> String {
        let start = self.start_day().map_or("na".into(), |d| d.to_string());
        format!("{}|{}|{start}", self.texts[CARD1], self.texts[ADDR1])
    }
}

#[derive(Default)]
struct Moments {
    n: u32,
    sum: f64,
    sum_sq: f64,
}

impl Moments {
    fn add(&mut self, x: f32) {
        if !x.is_nan() {
            self.n += 1;
            self.sum += x as f64;
            self.sum_sq += (x as f64).powi(2);
        }
    }

    fn mean(&self) -> Option<f32> {
        (self.n > 0).then(|| (self.sum / self.n as f64) as f32)
    }

    fn std(&self) -> Option<f32> {
        let mean = self.mean()? as f64;
        Some((self.sum_sq / self.n as f64 - mean * mean).max(0.0).sqrt() as f32)
    }
}

#[derive(Default)]
struct Accumulator {
    count: u32,
    amount: Moments,
    means: [Moments; 5],
    p_emails: HashSet<String>,
    r_emails: HashSet<String>,
    days: Option<(i64, i64)>,
}

impl Aggregates {
    pub fn build<'a>(
        rows: impl Iterator<Item = Row<'a>>,
        numeric_columns: &[String],
        categorical_columns: &[String],
    ) -> Result<Self> {
        let index = |columns: &[String], name: &'static str| {
            columns.iter().position(|c| c == name).ok_or(Error::MissingColumn(name))
        };
        let mut aggregates = Aggregates {
            numbers: try_map(NUMBERS, |n| index(numeric_columns, n))?,
            texts: try_map(TEXTS, |n| index(categorical_columns, n))?,
            customers: HashMap::new(),
            counts: Default::default(),
        };

        let mut accumulators: HashMap<String, Accumulator> = HashMap::new();
        for row in rows {
            let keys = aggregates.keys(&row);
            for (counts, value) in aggregates.counts.iter_mut().zip(keys.texts) {
                if !value.is_empty() {
                    *counts.entry(value.to_owned()).or_default() += 1;
                }
            }
            let acc = accumulators.entry(keys.customer()).or_default();
            acc.count += 1;
            acc.amount.add(keys.numbers[AMOUNT]);
            for (moments, &x) in acc.means.iter_mut().zip(&keys.numbers[2..]) {
                moments.add(x);
            }
            for (emails, value) in [(&mut acc.p_emails, keys.texts[P_EMAIL]), (&mut acc.r_emails, keys.texts[R_EMAIL])] {
                if !value.is_empty() {
                    emails.insert(value.to_owned());
                }
            }
            let (first, last) = acc.days.unwrap_or((keys.day, keys.day));
            acc.days = Some((first.min(keys.day), last.max(keys.day)));
        }

        aggregates.customers = accumulators
            .into_iter()
            .map(|(id, acc)| {
                let customer = Customer {
                    count: acc.count,
                    amount_mean: acc.amount.mean(),
                    amount_std: acc.amount.std(),
                    means: acc.means.each_ref().map(Moments::mean),
                    p_emails: acc.p_emails.len() as u32,
                    r_emails: acc.r_emails.len() as u32,
                    day_span: acc.days.map_or(0, |(first, last)| last - first),
                };
                (id, customer)
            })
            .collect();
        Ok(aggregates)
    }

    pub fn customers(&self) -> usize {
        self.customers.len()
    }

    pub fn features(&self, row: &Row) -> [f32; NAMES.len()] {
        let keys = self.keys(row);
        let mut f = [f32::NAN; NAMES.len()];
        match self.customers.get(&keys.customer()) {
            Some(c) => {
                let ratio = c.amount_mean.filter(|&m| m > 0.0).map(|m| keys.numbers[AMOUNT] / m);
                f[0] = c.count as f32;
                f[1] = nan_if_none(c.amount_mean);
                f[2] = nan_if_none(c.amount_std);
                f[3] = nan_if_none(ratio);
                for (k, mean) in c.means.iter().enumerate() {
                    f[4 + k] = nan_if_none(*mean);
                }
                f[9] = c.p_emails as f32;
                f[10] = c.r_emails as f32;
                f[11] = c.day_span as f32;
            }
            None => f[0] = 0.0,
        }
        f[12] = nan_if_none(keys.start_day().map(|d| d as f32));
        for (k, (counts, value)) in self.counts.iter().zip(keys.texts).enumerate() {
            if !value.is_empty() {
                f[13 + k] = counts.get(value).copied().unwrap_or(0) as f32;
            }
        }
        f
    }

    fn keys<'a>(&self, row: &Row<'a>) -> Keys<'a> {
        Keys {
            numbers: self.numbers.map(|i| row.numeric[i]),
            texts: self.texts.map(|i| row.categorical[i]),
            day: row.time.div_euclid(86_400),
        }
    }
}

fn nan_if_none(value: Option<f32>) -> f32 {
    value.unwrap_or(f32::NAN)
}

fn try_map<const N: usize>(names: [&'static str; N], f: impl Fn(&'static str) -> Result<usize>) -> Result<[usize; N]> {
    let mut out = [0; N];
    for (slot, name) in out.iter_mut().zip(names) {
        *slot = f(name)?;
    }
    Ok(out)
}
