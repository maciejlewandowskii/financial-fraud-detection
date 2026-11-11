use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use csv::{ByteRecord, StringRecord};
use serde::Serialize;

const LABEL: &str = "isFraud";
const ID: &str = "TransactionID";

pub type Transaction = HashMap<String, String>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error("brak kolumny {0}")]
    MissingColumn(&'static str),
    #[error("plik nie zawiera rekordów")]
    NoRows,
    #[error("brak rekordów do wylosowania")]
    NothingToPick,
    #[error("wiersz {row} ma {found} kolumn zamiast {expected}")]
    ColumnCount { row: usize, found: usize, expected: usize },
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Dataset {
    path: PathBuf,
    headers: StringRecord,
    label_col: usize,
    id_col: usize,
    offsets: Vec<u64>,
    fraud_rows: Vec<u32>,
    identity: Option<Identity>,
}

struct Identity {
    headers: StringRecord,
    id_col: usize,
    rows: HashMap<String, StringRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetInfo {
    pub path: String,
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
    /* Opens `train_transaction.csv` joined with `train_identity.csv`, as training reads them. */
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let path = dir.as_ref().join("train_transaction.csv");
        let mut reader = csv::Reader::from_path(&path)?;
        let headers = reader.headers()?.clone();
        let label_col = headers.iter().position(|h| h == LABEL).ok_or(Error::MissingColumn(LABEL))?;
        let id_col = headers.iter().position(|h| h == ID).ok_or(Error::MissingColumn(ID))?;

        let mut offsets = Vec::new();
        let mut fraud_rows = Vec::new();
        let mut record = ByteRecord::new();
        while reader.read_byte_record(&mut record)? {
            if record.get(label_col).is_some_and(is_true) {
                fraud_rows.push(offsets.len() as u32);
            }
            offsets.push(record.position().expect("csv reader tracks positions").byte());
        }
        if offsets.is_empty() {
            return Err(Error::NoRows);
        }

        let identity_path = dir.as_ref().join("train_identity.csv");
        let identity = if identity_path.exists() { Some(Identity::read(&identity_path)?) } else { None };
        let dataset = Self { path, headers, label_col, id_col, offsets, fraud_rows, identity };
        dataset.read_rows([0])?;
        Ok(dataset)
    }

    pub fn info(&self) -> DatasetInfo {
        DatasetInfo {
            path: self.path.display().to_string(),
            rows: self.offsets.len(),
            fraud_rows: self.fraud_rows.len(),
            columns: self.columns().map(str::to_owned).collect(),
        }
    }

    pub fn page(&self, offset: usize, limit: usize, only_fraud: bool) -> Result<Page> {
        let total = self.count(only_fraud);
        let start = offset.min(total);
        let end = offset.saturating_add(limit).min(total);
        let rows = self.read_rows((start..end).map(|i| self.resolve(i, only_fraud)))?;
        Ok(Page { offset: start, total, rows })
    }

    pub fn random(&self, only_fraud: bool) -> Result<RandomRow> {
        let total = self.count(only_fraud);
        if total == 0 {
            return Err(Error::NothingToPick);
        }
        let position = fastrand::usize(..total);
        let row = self.read_rows([self.resolve(position, only_fraud)])?.remove(0);
        Ok(RandomRow { position, row })
    }

    fn columns(&self) -> impl Iterator<Item = &str> {
        let transaction = self.headers.iter().enumerate().filter(|(i, _)| *i != self.label_col).map(|(_, h)| h);
        let identity = self.identity.iter().flat_map(|identity| identity.columns().map(|(_, h)| h));
        transaction.chain(identity)
    }

    fn count(&self, only_fraud: bool) -> usize {
        if only_fraud { self.fraud_rows.len() } else { self.offsets.len() }
    }

    fn resolve(&self, position: usize, only_fraud: bool) -> usize {
        if only_fraud { self.fraud_rows[position] as usize } else { position }
    }

    fn read_rows(&self, indices: impl IntoIterator<Item = usize>) -> Result<Vec<DatasetRow>> {
        let mut file = BufReader::new(File::open(&self.path)?);
        let mut line = Vec::new();
        let mut rows = Vec::new();
        for index in indices {
            file.seek(SeekFrom::Start(self.offsets[index]))?;
            line.clear();
            file.read_until(b'\n', &mut line)?;
            let record = parse_line(&line)?;
            if record.len() != self.headers.len() {
                return Err(Error::ColumnCount { row: index + 1, found: record.len(), expected: self.headers.len() });
            }
            let mut transaction: Transaction = self
                .headers
                .iter()
                .zip(record.iter())
                .enumerate()
                .filter(|(i, _)| *i != self.label_col)
                .map(|(_, (h, v))| (h.to_owned(), v.to_owned()))
                .collect();
            if let Some(identity) = &self.identity
                && let Some(values) = identity.rows.get(&record[self.id_col])
            {
                transaction.extend(identity.columns().map(|(i, h)| (h.to_owned(), values[i].to_owned())));
            }
            let is_fraud = is_true(record[self.label_col].trim().as_bytes());
            rows.push(DatasetRow { index, transaction, is_fraud });
        }
        Ok(rows)
    }
}

impl Identity {
    fn read(path: &Path) -> Result<Self> {
        let mut reader = csv::Reader::from_path(path)?;
        let headers = reader.headers()?.clone();
        let id_col = headers.iter().position(|h| h == ID).ok_or(Error::MissingColumn(ID))?;
        let mut rows = HashMap::new();
        for record in reader.records() {
            let record = record?;
            rows.insert(record[id_col].to_owned(), record);
        }
        Ok(Self { headers, id_col, rows })
    }

    fn columns(&self) -> impl Iterator<Item = (usize, &str)> {
        self.headers.iter().enumerate().filter(|(i, _)| *i != self.id_col)
    }
}

fn is_true(value: &[u8]) -> bool {
    value.eq_ignore_ascii_case(b"true") || value == b"1"
}

fn parse_line(line: &[u8]) -> Result<StringRecord> {
    let mut reader = csv::ReaderBuilder::new().has_headers(false).from_reader(line);
    Ok(reader.records().next().unwrap_or_else(|| Ok(StringRecord::new()))?)
}
