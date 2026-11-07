# Wykrywanie oszustw w transakcjach

Projekt SI (L3, zespół 8). Opis tematu: [karta-tematu.md](karta-tematu.md).

## Moduły

| Ścieżka                  | Co robi                                                                              |
| ------------------------ | ------------------------------------------------------------------------------------ |
| `crates/fraud-core`      | Wspólne typy, interfejs modelu, odczyt datasetu CSV, tymczasowa heurystyka zamiast modelu |
| `app/src-tauri`          | Backend aplikacji desktopowej (Tauri)                                                |
| `app/ui`                 | Interfejs: przeglądanie datasetu, losowanie rekordu, formularz i wynik modelu        |
| `sample/ieee-sample.csv` | 12 przykładowych rekordów w formacie IEEE-CIS do testów                              |

## Uruchomienie UI

Potrzebne: Rust i Node.js.

```sh
cd app
npm install
npm run tauri dev
```

W aplikacji kliknij „Wybierz CSV” i wskaż `train_transaction.csv` z [IEEE-CIS Fraud Detection](https://www.kaggle.com/competitions/ieee-fraud-detection) albo `sample/ieee-sample.csv`. Pełny dataset trzymaj poza repo albo w `data/` (ignorowane przez git).
