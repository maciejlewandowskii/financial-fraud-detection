use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Transaction(pub HashMap<String, String>);

impl Transaction {
    pub fn text(&self, column: &str) -> Option<&str> {
        self.0
            .get(column)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
    }

    pub fn number(&self, column: &str) -> Option<f64> {
        self.text(column)?.parse().ok()
    }
}
