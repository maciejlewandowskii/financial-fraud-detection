use std::collections::HashMap;
use std::path::Path;

use csv::StringRecord;

use crate::{Error, Result};

pub const ID: &str = "TransactionID";
pub const LABEL: &str = "isFraud";
pub const TIME: &str = "TransactionDT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Skip,
    Numeric,
    Categorical,
}

pub fn role(column: &str) -> Role {
    let numbered = |prefix: &str, range: std::ops::RangeInclusive<u32>| {
        column.strip_prefix(prefix).and_then(|n| n.parse().ok()).is_some_and(|n| range.contains(&n))
    };
    match column {
        ID | LABEL | TIME => Role::Skip,
        "ProductCD" | "addr1" | "addr2" | "P_emaildomain" | "R_emaildomain" | "DeviceType" | "DeviceInfo" => {
            Role::Categorical
        }
        _ if numbered("card", 1..=6) || numbered("M", 1..=9) || numbered("id_", 12..=38) => Role::Categorical,
        _ => Role::Numeric,
    }
}

pub fn parse_number(value: &str) -> f32 {
    value.trim().parse().unwrap_or(f32::NAN)
}

#[derive(Clone, Copy)]
enum Source {
    Transaction(usize),
    Identity(usize),
}

/* Missing numbers are NaN, missing texts are empty. */
pub struct Row<'a> {
    pub time: i64,
    pub numeric: &'a [f32],
    pub categorical: Vec<&'a str>,
}

pub struct Table {
    pub numeric_columns: Vec<String>,
    pub categorical_columns: Vec<String>,
    pub times: Vec<i64>,
    pub labels: Vec<bool>,
    numeric: Vec<f32>,
    categorical: Vec<u32>,
    values: Vec<Vec<String>>,
}

impl Table {
    pub fn read(dir: &Path, limit: Option<usize>) -> Result<Self> {
        let identity_path = dir.join("train_identity.csv");
        let (identity_header, identity) = if identity_path.exists() {
            read_identity(&identity_path)?
        } else {
            (Vec::new(), HashMap::new())
        };

        let mut reader = open(&dir.join("train_transaction.csv"))?;
        let header = normalized_header(&mut reader)?;
        let find = |name| header.iter().position(|c| c == name).ok_or(Error::MissingColumn(name));
        let (id_col, label_col, time_col) = (find(ID)?, find(LABEL)?, find(TIME)?);

        let transaction_columns = header.iter().enumerate().map(|(i, c)| (Source::Transaction(i), c));
        let identity_columns = identity_header.iter().enumerate().map(|(i, c)| (Source::Identity(i), c));
        let mut numeric_src = Vec::new();
        let mut categorical_src = Vec::new();
        for (source, column) in transaction_columns.chain(identity_columns) {
            match role(column) {
                Role::Numeric => numeric_src.push((source, column.clone())),
                Role::Categorical => categorical_src.push((source, column.clone())),
                Role::Skip => {}
            }
        }

        let mut table = Table {
            numeric_columns: numeric_src.iter().map(|(_, c)| c.clone()).collect(),
            categorical_columns: categorical_src.iter().map(|(_, c)| c.clone()).collect(),
            times: Vec::new(),
            labels: Vec::new(),
            numeric: Vec::new(),
            categorical: Vec::new(),
            values: vec![Vec::new(); categorical_src.len()],
        };
        let mut ids: Vec<HashMap<String, u32>> = vec![HashMap::new(); categorical_src.len()];
        let empty = StringRecord::new();
        let mut record = StringRecord::new();
        while limit.is_none_or(|l| table.len() < l) && reader.read_record(&mut record)? {
            let identity = identity.get(&record[id_col]).unwrap_or(&empty);
            let get = |(source, _): &(Source, String)| match *source {
                Source::Transaction(i) => record.get(i).unwrap_or(""),
                Source::Identity(i) => identity.get(i).unwrap_or(""),
            };
            table.labels.push(match &record[label_col] {
                "1" => true,
                "0" => false,
                other => return Err(Error::InvalidValue { column: LABEL, value: other.to_owned() }),
            });
            let time = &record[time_col];
            table.times.push(time.parse().map_err(|_| Error::InvalidValue { column: TIME, value: time.to_owned() })?);
            table.numeric.extend(numeric_src.iter().map(|src| parse_number(get(src))));
            for (j, src) in categorical_src.iter().enumerate() {
                let value = get(src).trim();
                let id = match value {
                    "" => 0,
                    _ => *ids[j].entry(value.to_owned()).or_insert_with(|| {
                        table.values[j].push(value.to_owned());
                        table.values[j].len() as u32
                    }),
                };
                table.categorical.push(id);
            }
        }
        if table.labels.is_empty() {
            return Err(Error::NoRows);
        }
        Ok(table)
    }

    pub fn len(&self) -> usize {
        self.labels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn row(&self, i: usize) -> Row<'_> {
        let (m, k) = (self.numeric_columns.len(), self.categorical_columns.len());
        Row {
            time: self.times[i],
            numeric: &self.numeric[i * m..(i + 1) * m],
            categorical: self.categorical[i * k..(i + 1) * k]
                .iter()
                .enumerate()
                .map(|(j, &id)| if id == 0 { "" } else { self.values[j][id as usize - 1].as_str() })
                .collect(),
        }
    }
}

type Identity = (Vec<String>, HashMap<String, StringRecord>);

fn read_identity(path: &Path) -> Result<Identity> {
    let mut reader = open(path)?;
    let header = normalized_header(&mut reader)?;
    let id_col = header.iter().position(|c| c == ID).ok_or(Error::MissingColumn(ID))?;
    let mut rows = HashMap::new();
    for record in reader.records() {
        let record = record?;
        rows.insert(record[id_col].to_owned(), record);
    }
    Ok((header, rows))
}

fn open(path: &Path) -> Result<csv::Reader<std::fs::File>> {
    csv::Reader::from_path(path).map_err(|source| Error::Read { path: path.to_owned(), source })
}

fn normalized_header(reader: &mut csv::Reader<std::fs::File>) -> Result<Vec<String>> {
    Ok(reader.headers()?.iter().map(|c| c.trim().replace('-', "_")).collect())
}
