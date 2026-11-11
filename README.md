# Wykrywanie oszustw w transakcjach

Projekt SI (L3, zespół 8). Opis tematu: [karta-tematu.md](karta-tematu.md).

## Uruchomienie UI

```sh
cd app
npm install
npm run tauri dev
```
## Dane

Zbiór [IEEE-CIS Fraud Detection](https://www.kaggle.com/c/ieee-fraud-detection) z Kaggle. Wymaga zaakceptowania zasad konkursu na stronie oraz tokenu API w `~/.kaggle/kaggle.json`.

```sh
pip install kaggle
kaggle competitions download -c ieee-fraud-detection -p data/ieee
unzip data/ieee/ieee-fraud-detection.zip -d data/ieee
```

## Trening

```sh
cargo run -p fraud-train --release
```
