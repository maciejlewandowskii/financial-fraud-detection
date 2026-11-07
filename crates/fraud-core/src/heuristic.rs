use crate::{Error, FraudModel, Prediction, Transaction};

pub struct HeuristicModel;

impl FraudModel for HeuristicModel {
    fn name(&self) -> &str {
        "Heurystyka zastępcza (do czasu podpięcia sieci neuronowej)"
    }

    fn predict(&self, tx: &Transaction) -> Result<Prediction, Error> {
        let amount = tx
            .number("TransactionAmt")
            .ok_or("brak lub błędna kwota (TransactionAmt)")?;
        let risky_email = matches!(
            tx.text("P_emaildomain"),
            Some("protonmail.com" | "mail.com" | "outlook.es") | None
        );
        let z = -4.0
            + 0.35 * amount.max(0.0).ln_1p()
            + if tx.text("ProductCD") == Some("C") {
                1.5
            } else {
                0.0
            }
            + if tx.text("card6") == Some("credit") {
                0.7
            } else {
                0.0
            }
            + if risky_email { 0.8 } else { 0.0 }
            + 0.03 * tx.number("C1").unwrap_or(0.0).min(100.0);
        let probability = 1.0 / (1.0 + (-z).exp());
        Ok(Prediction {
            is_fraud: probability >= 0.5,
            fraud_probability: probability as f32,
        })
    }
}
