# Transaction Fraud Detection

AI course project — Collegium Da Vinci, Computer Science, group L3, academic year 2026/2027.

## What it does

A desktop app for online shops, payment providers and banks that spots fraudulent card transactions. It takes the data of one transaction (amount, time, card, address, e-mail domain, device and the anonymised columns from the dataset) and returns the probability of fraud, plus a decision: suspicious or fine. You can browse transactions from the dataset, pick a random one or type one in, and compare the model's verdict with the true label.

## AI technique

A neural network (a multilayer perceptron with embeddings for the categorical columns), trained on 590,540 labelled transactions from the IEEE-CIS Fraud Detection dataset. Fraud methods keep changing and most of the 430+ columns are anonymised, so rules can't be written by hand. A network learns them from examples and can simply be retrained on newer data.

## Results

| Test set (59,054 transactions) | Random baseline | Our model |
|---|---|---|
| AUC-PR | 0.035 | **0.793** |
| AUROC | 0.5 | 0.958 |

At the chosen threshold (0.945), precision is 98.4% and recall 45.4%: 938 frauds caught and 15 false alarms.

## How to run

Requirements:
- Rust (latest stable, at least 1.88),
- Node.js with npm,
- Tauri prerequisites (on Windows: Microsoft C++ Build Tools, see https://v2.tauri.app/start/prerequisites/),
- a few GB of RAM for training.

The data is the [IEEE-CIS Fraud Detection](https://www.kaggle.com/c/ieee-fraud-detection) dataset from Kaggle. You need a Kaggle account and you have to accept the competition rules. Downloading from the command line also needs Python with the `kaggle` package and an API token in `~/.kaggle/kaggle.json`.

```sh
git clone https://github.com/maciejlewandowskii/financial-fraud-detection.git
cd financial-fraud-detection

# data (into data/ieee)
pip install kaggle
kaggle competitions download -c ieee-fraud-detection -p data/ieee
unzip data/ieee/ieee-fraud-detection.zip -d data/ieee

# training (about 15-20 minutes on an M1 Pro MacBook)
cargo run -p fraud-train --release

# application
cd app
npm install
npm run tauri dev
```

Where the output appears:
- After training, the terminal shows AUC-PR, AUROC, precision, recall and the chosen threshold.
- The model and `report.json` with the results are saved to `artifacts/`.
- The app shows the model's AUC-PR in its header and scores a transaction when you click it in the table.

## Team

- **Maciej Lewandowski**: model (data and feature preparation, neural network, training, evaluation)
- **Viachaslau Lukashonak**: frontend (design and implementation of the Tauri desktop app and UI)
- **Marcel Krzysztoń**: project proposal form, documentation in Polish and English, presentation

## Documentation

- Full documentation in Polish: [`docs/dokumentacja.md`](docs/dokumentacja.md)
- Full documentation in English: [`docs/documentation_en.md`](docs/documentation_en.md)
- Technical abstract: [`docs/abstract_en.md`](docs/abstract_en.md)
- Project proposal form (in Polish): [`karta-tematu.md`](karta-tematu.md)
