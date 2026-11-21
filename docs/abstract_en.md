# Transaction Fraud Detection: Technical Abstract

**Team Z8, group L3:** Maciej Lewandowski, Viachaslau Lukashonak, Marcel Krzysztoń · Artificial Intelligence lab, Collegium Da Vinci, 2026/2027 · https://github.com/maciejlewandowskii/financial-fraud-detection

We built a desktop application that estimates how likely a card transaction is to be fraudulent. Checking every payment by hand is impossible, and hand-written rules quickly go out of date because fraudsters keep changing their methods, so we use a model that learns from labelled examples instead.

**Data.** We use the IEEE-CIS Fraud Detection dataset from Kaggle: 590,540 real e-commerce transactions from Vesta, 3.5% of them fraud, with over 430 mostly anonymised columns. We switched to it after our first dataset turned out to carry too little signal: the model reached an AUC-PR of 4.39% at a 3.58% fraud rate, barely better than guessing. Before training we:
- join the transaction and identity tables,
- log-scale and standardise numeric columns, adding flags for missing values,
- add cyclic time-of-day and day-of-week features,
- build vocabularies for categorical columns,
- compute 17 per-customer aggregates, where a customer is approximated by card1, addr1 and the card's start day derived from D1.

The data is split 80/10/10, stratified, with seed 42. All preprocessing statistics are fitted on the training part only.

**Model.** A multilayer perceptron (hidden layers 512, 256 and 128, ReLU, dropout 0.4) with 8-dimensional embeddings for the 49 categorical columns. It is trained for 40 epochs with binary cross-entropy and AdamW, entirely on CPU (Rust with the burn framework). Training takes about 15-20 minutes on an M1 Pro MacBook. The decision threshold is picked on the validation set by minimising 10·FP + FN, so a false alarm counts as ten times worse than a missed fraud.

**Results** (test set, 59,054 transactions):

| Metric | Value |
|---|---|
| AUC-PR | 0.793 (random baseline: 0.035, about 22× lower) |
| AUROC | 0.958 |
| Threshold | 0.945 |
| Precision at threshold | 98.4% |
| Recall at threshold | 45.4% |
| Frauds caught / false alarms | 938 / 15 |

**Limitations.** The split is random rather than time-based, and customer aggregates are computed on the whole dataset, so the test score is probably optimistic. Next steps would be a time-based split, keeping the best epoch instead of the last one, and a comparison with gradient boosting.

**Application.** A Tauri 2 desktop app lets the user browse dataset records, pick a random one or type in a transaction, and see the model's verdict next to the true label.
