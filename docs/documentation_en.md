# Transaction Fraud Detection

**Group:** L3 · **Team:** Z8 · **Repository:** https://github.com/maciejlewandowskii/financial-fraud-detection
Artificial Intelligence — laboratory · Collegium Da Vinci · academic year 2026/2027

## 1. Problem *(rubric A)*

Online shops and banks process huge numbers of card payments, and some of them are fraud, e.g. purchases made with a stolen card. It is impossible to check every transaction by hand, and a mistake in either direction is costly: a missed fraud means lost money, and a blocked honest payment means an annoyed customer. Our program takes the data of a single transaction (amount, time, product type, card details, address, e-mail domain, device and a lot of anonymised columns from the dataset). It returns the probability that the transaction is fraud, plus a decision: suspicious or fine. Everything runs as a desktop application where you can browse transactions from the dataset, or type in your own, and immediately see what the model thinks.

## 2. Why AI and why this technique *(B · 7 pts)*

A regular algorithm is not enough, because there are too many rules and they keep changing. Fraudsters constantly come up with new methods, so hand-written conditions like "amount above X and a new card" would quickly stop working. On top of that, the dataset has over 430 columns, most of them anonymised (V1-V339, C1-C14 etc.). We don't even know what these columns mean, so writing rules based on them is out of the question. The model learns these relationships by itself from labelled examples, and when fraudsters change their methods, it's enough to retrain it on newer data.

We chose a neural network (MLP), because it handles a large number of features at once well and finds non-linear relationships between them on its own. Text columns with thousands of distinct values (e.g. the card number card1, e-mail domains) are turned into embeddings, i.e. short vectors of numbers that the network learns together with everything else. If we had gone with an expert system, we would have had to come up with the rules ourselves, and with anonymised columns there is nothing to derive them from.

## 3. Data *(B · 8 pts)*

We use the IEEE-CIS Fraud Detection dataset from Kaggle (https://www.kaggle.com/c/ieee-fraud-detection). These are real e-commerce transactions shared by the company Vesta. To download them, you need a Kaggle account and you have to accept the competition rules.

It wasn't our first choice. At the beginning we took the Financial Transactions Dataset for Fraud Detection, also from Kaggle, where fraud makes up 3.58% of transactions. The first model trained on it reached an AUC-PR of 4.39%, i.e. only about 1.23 times better than guessing. Further training didn't help: the very first epoch gave the same results as the following ones, so it wasn't an overfitting problem. There simply wasn't enough information in that data to tell fraud apart from normal transactions. Discussions on Kaggle about that dataset also said this is practically the upper limit. That's why we switched to IEEE-CIS, which has much more information about each transaction (over 430 columns). On it, our model reaches an AUC-PR of 0.79, i.e. over 22 times better than guessing (section 5).

The whole dataset is about 1.35 GB of CSV files after unzipping (the download is a ZIP archive of about 118 MB). We use two files from it:
- `train_transaction.csv`: 590,540 transactions, 394 columns, including the `isFraud` label,
- `train_identity.csv`: additional information about the device and browser, but only for some of the transactions.

We skipped the Kaggle test files, because they have no labels. Fraud is only about 3.5% of the data, so the classes are heavily imbalanced, which affected our choice of metric (section 5). The repo also contains `sample/ieee-sample.csv` with 12 rows, to show what the data looks like.

What we do with the data before training:
1. We join both files on `TransactionID`. If a transaction has no device data, those fields stay empty.
2. We split the columns into numeric (382) and categorical (49, e.g. ProductCD, card1-card6, e-mail domains, M1-M9, DeviceInfo).
3. We add 17 features about the "customer". From analyses done during the competition we know that D1 is roughly the number of days since the card was first used, so the transaction day minus D1 is constant for a given card. Together with card1 and addr1, this gives something like a customer ID. For each customer we compute e.g. how many transactions they have, what their average amount is and whether the current amount differs a lot from it.
4. Numbers go through a logarithm, because amounts have a very long tail: most are a few dozen dollars, but there are thousands too. Then we standardise them. Missing values are replaced with 0, and we add flags saying the value was missing.
5. From the transaction time we extract the time of day and the day of the week, and from the amount the cents (the part after the decimal point).
6. For categorical columns we build a vocabulary of values that appeared at least 10 times. Rarer values go into a shared "unknown" group.

We split the data randomly: 80% training, 10% validation, 10% test (472,432 / 59,054 / 59,054 transactions), with the same fraud rate in each part. The seed is 42, so the split comes out the same every time.

The test set doesn't see the training data, because they are separate rows. Everything that is "learned" during data preparation (means for standardisation, category vocabularies) is computed on the training part only. We pick the decision threshold on validation, and the test set is run once, at the very end. There is one caveat: the customer features from step 3 are computed on the whole dataset. They don't use labels, so they don't give away the answer, but the same card can appear in both training and test (more in section 7).

## 4. Solution *(A · 10 pts, B · 5 pts)*

### How it works

CSV data → feature preparation → network training → saved model (`artifacts/`) → application

1. The training program (`fraud-train`) loads the data, splits it into parts and prepares the features.
2. The network trains for 40 epochs. After each epoch, the validation results are shown in the terminal.
3. On validation we choose the threshold from which a transaction is considered fraud.
4. On the test set we compute the final results and save everything to the `artifacts/` folder: the model weights, the data preparation settings and a report with the results.
5. The application (Tauri) loads this model and scores transactions picked from the table or typed in by hand.

Everything is written in Rust. The network is built with the burn library and runs on CPU, and the interface is plain HTML and JavaScript.

### Model

The network is an MLP with three hidden layers (512, 256 and 128 neurons, ReLU activation). Its input is the prepared numeric features and the embeddings of the categorical columns: each category value gets its own vector of 8 numbers. At the end there is a single neuron. We pass its output through a sigmoid to get the probability of fraud:

```
p = 1 / (1 + e^(-z))
```

where z is the raw output of the last neuron.

The network learns by minimising cross-entropy (binary cross-entropy):

```
L = -[ y * ln(p) + (1 - y) * ln(1 - p) ]
```

y = 1 for fraud, 0 for a normal transaction. The more confident the model is when it's wrong, the bigger the penalty. If it gave a fraud a 1% chance, the loss is about 4.6, and at 50% only about 0.7. The gradient of this loss with respect to z turns out to be simply p - y, i.e. exactly how much the model was off. We update the weights with gradient descent (the AdamW optimiser), and the learning rate is gradually decreased during training. To keep the network from memorising the data, we use dropout, i.e. during training we randomly switch off some of the neurons.

### Decision threshold

The network only returns a probability, but the application has to say "yes" or "no". We pick the threshold on validation so that the cost of errors is as low as possible:

```
cost = 10 * false alarms + 1 * missed frauds
```

We assumed that blocking an honest customer is 10 times worse than letting a fraud through. That's why the model flags fraud only when it is really sure. In our case the threshold came out at 0.945, i.e. a transaction is marked as fraud only when the chance is above 94%. The weight of a false alarm can be changed with the `--fp-cost` parameter.

### Key parameters

| Parameter | Value |
|---|---|
| Hidden layers | 512, 256, 128 |
| Embedding size | 8 |
| Dropout | 0.4 (0.1 on the input) |
| Epochs | 40 |
| Batch size | 4096 |
| Learning rate | 0.001, decreasing to 0.00001 |
| Weight decay | 0.01 |
| Data split | 80 / 10 / 10 |
| Seed | 42 |
| False alarm cost | 10 |

## 5. Evaluation *(B · 8 pts)*

The main metric is AUC-PR, i.e. the area under the precision-recall curve. Accuracy doesn't work here: with 3.5% fraud, a model that always says "everything's fine" has 96.5% accuracy and doesn't catch a single fraud. AUC-PR checks how well the model picks out exactly this rare class. We also report AUROC, and precision and recall at the chosen threshold.

Success criterion from the project proposal form: AUC-PR ≥ 0.75 on the test set.

The baseline is a random model. Its AUC-PR is equal to the fraud rate, i.e. about 0.035, and its AUROC is 0.5. For precision and recall we compare against a model that always says "fine" and doesn't catch any fraud.

| Case | Baseline | Our solution |
|---|---|---|
| Test (59,054 transactions, 2,067 frauds): AUC-PR | 0.035 | **0.793** |
| Test: AUROC | 0.5 | 0.958 |
| Test: precision / recall at threshold | - / 0% | 98.4% / 45.4% |
| Test: frauds caught / false alarms | 0 / 0 | 938 / 15 |
| Validation (59,054 transactions): AUC-PR | 0.035 | 0.779 |
| Chosen threshold | - | 0.945 |

Conclusion: on the test set the model has an AUC-PR of 0.79, i.e. over 22 times more than a random model, so it meets the criterion from the project proposal form (≥ 0.75). The validation result (0.78) is very close, so it's not a fluke of one particular set. At the chosen threshold the model catches 938 out of 2,067 frauds (45%) and raises only 15 false alarms on almost 57 thousand honest transactions, so, as intended in section 4, it is very cautious.

## 6. Non-functional requirements *(A · 5 pts)*

Running time (CPU):
- training on the full data (40 epochs) took about 15-20 minutes on a MacBook Pro with an M1 Pro chip. The program prints the time itself at the end,
- to quickly check that everything works, you can run training on a chunk of the data: `--limit 50000 --epochs 5`,
- scoring a single transaction in the application is practically instant,
- training keeps the whole table in memory, so it needs a few GB of RAM.

Bad or incomplete data:
- if the data file or a required column is missing, or a label has an odd value, training stops with a message saying what's wrong (which column, which value),
- `train_identity.csv` is optional. Without it, the model learns from transactions only,
- empty fields are normal in this dataset, so the model handles them: a missing number is treated as the mean and marked with a flag, and a missing category has its own value,
- an unknown category (e.g. an e-mail domain that wasn't in the training data) doesn't cause an error, it just goes into the "unknown" group,
- the application starts even without a trained model and shows a message with the command to train it.

Reproducibility: seed 42 is hard-coded. It fixes the data split, the shuffling and the initial network weights, so a rerun should give practically the same result. We can't guarantee an identical result down to the last digit, because we load data in 4 threads and the order of batches may differ slightly. We trained on the full data only once, so we haven't compared the results of two runs. To check it, it's enough to run training a second time and compare the `report.json` files.

## 7. Limitations *(E · 4 pts)*

- **Random split instead of a time-based one.** The data covers about half a year, so the model also learns from transactions that are "from the future" relative to parts of the test set. The same card can also end up in both training and test. Because of this, the test score is probably a bit better than it would be in real use. It would be better to train on the first months and test on the last one.
- **New customers and rare data.** The worst cases are new cards that weren't in the data (the customer features are empty then), transactions without device data, and rare categories.
- **Cautious threshold.** Because of the high cost of a false alarm, a lot of fraud gets through: on the test set the model missed 1,129 out of 2,067 (55%). A bank that is more worried about losses than about unhappy customers should lower `--fp-cost`.
- **Model from the last epoch.** We save the model after the last epoch, not the one that had the best validation score.
- **No explanations.** The model doesn't explain why it considered a transaction suspicious, and with anonymised columns it's hard to even check that by hand.
- **Model ageing.** Over time it has to be retrained. It also only works on data in the IEEE-CIS format.
- **Moving to another computer.** `report.json` stores the full path to the data, so after copying the model to another computer, the transaction browser in the application won't find it. Scoring transactions still works.

Next steps: a time-based split, saving the best model from validation, a comparison with LightGBM/XGBoost (the best solutions in this competition were built on them) and explaining the model's decisions, e.g. with SHAP.

## 8. Real-world applications *(E · 4 pts)*

- **Visa and Mastercard.** For every card payment, a model assesses in real time how risky the transaction is, and the bank takes that score into account when approving it (Visa Advanced Authorization, Mastercard Decision Intelligence).
- **Stripe and PayPal.** Models trained on payments from many shops at once (e.g. Stripe Radar) block suspicious payments or require extra verification, e.g. 3-D Secure.
- **Vesta.** The company our dataset comes from does exactly this for online shops: it assesses the risk of every transaction and guarantees to cover fraud losses.

## 9. Team *(D · 8 pts)*

| Person | Responsible for |
|---|---|
| Maciej Lewandowski | model: data and feature preparation, neural network, training, evaluation |
| Viachaslau Lukashonak | frontend: design and implementation of the desktop application (Tauri) and the user interface |
| Marcel Krzysztoń | project proposal form, documentation in PL and EN, presentation |

In the project proposal form, Maciej and Viachaslau shared data and implementation. In practice they split the work so that Maciej took the model and Viachaslau the frontend. Each of us wrote sprint reports. The reports from sprints 7 and 8 are in the `sprints/` folder.

## 10. Running the project *(C · 4 pts)*

Requirements:
- Rust, latest stable version, at least 1.88 (https://rustup.rs),
- Node.js with npm,
- on Windows, Microsoft C++ Build Tools (other Tauri requirements: https://v2.tauri.app/start/prerequisites/),
- a few GB of free RAM for training,
- to download the data: Python with the `kaggle` package and an API token in `~/.kaggle/kaggle.json`. You can also download the data manually from the competition page with the "Download All" button.

```sh
# 1. Repository
git clone https://github.com/maciejlewandowskii/financial-fraud-detection.git
cd financial-fraud-detection

# 2. Data (into the data/ieee folder)
pip install kaggle
kaggle competitions download -c ieee-fraud-detection -p data/ieee
unzip data/ieee/ieee-fraud-detection.zip -d data/ieee
# on Windows, instead of unzip: Expand-Archive data/ieee/ieee-fraud-detection.zip data/ieee

# 3. Training
cargo run -p fraud-train --release
# quick test: cargo run -p fraud-train --release -- --limit 50000 --epochs 5

# 4. Application
cd app
npm install
npm run tauri dev
```

Where the results are:
- in the terminal after training: AUC-PR, AUROC, precision, recall and the chosen threshold,
- in the `artifacts/` folder: the model and `report.json` with the results,
- in the application: the model's AUC-PR is shown in the header, and after clicking a transaction in the table (or the "Losuj" / random button) the model shows its verdict next to the true label. The interface is in Polish.

Training options: `--data` (data folder, default `data/ieee`), `--out` (where to save the model, default `artifacts`), `--epochs` (default 40), `--limit` (only the first N transactions), `--fp-cost` (default 10). The application can load the model from a different folder via the `FRAUD_MODEL_DIR` environment variable.

## 11. AI tools

Maciej used AI for the model part:
- as help with code review: the AI compared our code with example projects from GitHub and pointed out mistakes,
- for the analysis of why the first dataset we chose (Financial Transactions Dataset for Fraud Detection from Kaggle) wasn't a good fit for the project. After this analysis we switched to IEEE-CIS,
- for feature normalisation: there are several hundred columns, so choosing transformations for each of them by hand would have been very time-consuming.

Viachaslau designed and built the Tauri interface mostly on his own. AI only helped him with small things:
- minor code fixes,
- preparing sample data to check how the UI looks and works,
- temporary placeholder commands (mocks), which were later replaced by the real logic with the model.

We used Claude (Claude Code) for writing the documentation. It helped analyse the code and prepare a first draft of the text, which we then corrected and filled in with the results.
