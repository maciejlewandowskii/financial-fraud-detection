use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use csv::{ByteRecord, StringRecord};
use serde::Serialize;

use crate::{Error, Transaction};

const LABEL_COLUMNS: [&str; 2] = ["isFraud", "is_fraud"];

pub struct Dataset {
    path: PathBuf,
    headers: StringRecord,
    label_col: usize,
    offsets: Vec<u64>,
    fraud_rows: Vec<u32>,
    size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetInfo {
    pub path: String,
    pub size_bytes: u64,
    pub rows: usize,
    pub fraud_rows: usize,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DatasetRow {
    pub index: usize,
    pub transaction: Transaction,
    pub is_fraud: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RandomRow {
    pub position: usize,
    pub row: DatasetRow,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub offset: usize,
    pub total: usize,
    pub rows: Vec<DatasetRow>,
}

impl Dataset {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let size_bytes = std::fs::metadata(&path)?.len();
        let mut reader = csv::Reader::from_path(&path)?;
        let headers = reader.headers()?.clone();
        let label_col = headers
            .iter()
            .position(|h| LABEL_COLUMNS.contains(&h))
            .ok_or("brak kolumny isFraud")?;

        let mut offsets = Vec::new();
        let mut fraud_rows = Vec::new();
        let mut record = ByteRecord::new();
        while reader.read_byte_record(&mut record)? {
            let position = record.position().ok_or("brak pozycji rekordu")?;
            if record.get(label_col).is_some_and(is_true) {
                fraud_rows.push(offsets.len() as u32);
            }
            offsets.push(position.byte());
        }
        if offsets.is_empty() {
            return Err("plik nie zawiera rekordów".into());
        }

        let dataset = Self {
            path,
            headers,
            label_col,
            offsets,
            fraud_rows,
            size_bytes,
        };
        dataset.read_rows([0])?;
        Ok(dataset)
    }

    pub fn info(&self) -> DatasetInfo {
        DatasetInfo {
            path: self.path.display().to_string(),
            size_bytes: self.size_bytes,
            rows: self.offsets.len(),
            fraud_rows: self.fraud_rows.len(),
            columns: self.columns().map(str::to_owned).collect(),
        }
    }

    pub fn page(&self, offset: usize, limit: usize, only_fraud: bool) -> Result<Page, Error> {
        let total = self.count(only_fraud);
        let start = offset.min(total);
        let end = offset.saturating_add(limit).min(total);
        let rows = self.read_rows((start..end).map(|i| self.resolve(i, only_fraud)))?;
        Ok(Page {
            offset: start,
            total,
            rows,
        })
    }

    pub fn random(&self, only_fraud: bool) -> Result<RandomRow, Error> {
        let total = self.count(only_fraud);
        if total == 0 {
            return Err("brak rekordów do wylosowania".into());
        }
        let position = fastrand::usize(..total);
        let row = self
            .read_rows([self.resolve(position, only_fraud)])?
            .remove(0);
        Ok(RandomRow { position, row })
    }

    fn columns(&self) -> impl Iterator<Item = &str> {
        self.headers
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != self.label_col)
            .map(|(_, h)| h)
    }

    fn count(&self, only_fraud: bool) -> usize {
        if only_fraud {
            self.fraud_rows.len()
        } else {
            self.offsets.len()
        }
    }

    fn resolve(&self, position: usize, only_fraud: bool) -> usize {
        if only_fraud {
            self.fraud_rows[position] as usize
        } else {
            position
        }
    }

    fn read_rows(
        &self,
        indices: impl IntoIterator<Item = usize>,
    ) -> Result<Vec<DatasetRow>, Error> {
        let mut file = BufReader::new(File::open(&self.path)?);
        let mut line = Vec::new();
        let mut rows = Vec::new();
        for index in indices {
            file.seek(SeekFrom::Start(self.offsets[index]))?;
            line.clear();
            file.read_until(b'\n', &mut line)?;
            let record = parse_line(&line)?;
            if record.len() != self.headers.len() {
                return Err(format!(
                    "wiersz {} ma {} kolumn zamiast {}",
                    index + 1,
                    record.len(),
                    self.headers.len()
                )
                .into());
            }
            let fields = self
                .headers
                .iter()
                .zip(record.iter())
                .enumerate()
                .filter(|(i, _)| *i != self.label_col)
                .map(|(_, (h, v))| (h.to_owned(), v.to_owned()))
                .collect();
            rows.push(DatasetRow {
                index,
                transaction: Transaction(fields),
                is_fraud: is_true(record[self.label_col].trim().as_bytes()),
            });
        }
        Ok(rows)
    }
}

fn is_true(value: &[u8]) -> bool {
    value.eq_ignore_ascii_case(b"true") || value == b"1"
}

fn parse_line(line: &[u8]) -> Result<StringRecord, Error> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(line)
        .records()
        .next()
        .ok_or("pusty wiersz CSV")?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Dataset {
        Dataset::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sample/ieee-sample.csv"))
            .unwrap()
    }

    fn ids(page: &Page) -> Vec<&str> {
        page.rows
            .iter()
            .map(|r| r.transaction.text("TransactionID").unwrap())
            .collect()
    }

    #[test]
    fn indexes_rows_fraud_and_columns() {
        let info = sample().info();
        assert_eq!((info.rows, info.fraud_rows), (12, 3));
        assert_eq!(info.columns.len(), 393);
        assert_eq!(
            &info.columns[..3],
            ["TransactionID", "TransactionDT", "TransactionAmt"]
        );
    }

    #[test]
    fn pages_through_all_rows() {
        let dataset = sample();
        let first = dataset.page(0, 5, false).unwrap();
        assert_eq!(
            ids(&first),
            ["2987000", "2987001", "2987002", "2987003", "2987004"]
        );
        assert_eq!(first.total, 12);
        let last = dataset.page(10, 5, false).unwrap();
        assert_eq!(ids(&last), ["2987010", "2987011"]);
        assert_eq!(last.rows[0].index, 10);
        assert!(dataset.page(100, 5, false).unwrap().rows.is_empty());
    }

    #[test]
    fn filters_fraud_rows() {
        let dataset = sample();
        let page = dataset.page(0, 10, true).unwrap();
        assert_eq!(ids(&page), ["2987004", "2987007", "2987010"]);
        assert!(page.rows.iter().all(|r| r.is_fraud));
        for _ in 0..50 {
            let picked = dataset.random(true).unwrap();
            assert!(picked.row.is_fraud);
            assert_eq!(page.rows[picked.position], picked.row);
        }
    }

    #[test]
    fn reads_fields_and_empty_values() {
        let row = sample().page(0, 1, false).unwrap().rows.remove(0);
        let tx = &row.transaction;
        assert_eq!(tx.number("TransactionAmt"), Some(68.5));
        assert_eq!(tx.text("card4"), Some("discover"));
        assert_eq!(tx.text("card2"), None);
        assert!(!tx.0.contains_key("isFraud"));
        assert!(!row.is_fraud);
    }

    #[test]
    fn rejects_bad_files() {
        let dir = std::env::temp_dir();
        let header_only = dir.join("fraud-core-header-only.csv");
        std::fs::write(&header_only, "TransactionID,isFraud\n").unwrap();
        assert!(Dataset::open(&header_only).is_err());
        let no_label = dir.join("fraud-core-no-label.csv");
        std::fs::write(&no_label, "TransactionID,TransactionAmt\n1,2\n").unwrap();
        assert!(Dataset::open(&no_label).is_err());
    }

    #[test]
    fn heuristic_scores_sample() {
        use crate::{FraudModel, HeuristicModel};
        let dataset = sample();
        for row in dataset.page(0, 12, false).unwrap().rows {
            let p = HeuristicModel.predict(&row.transaction).unwrap();
            assert!((0.0..=1.0).contains(&p.fraud_probability));
        }
        assert!(HeuristicModel.predict(&Transaction::default()).is_err());
    }
}
