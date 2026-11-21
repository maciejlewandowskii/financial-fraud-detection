# Wykrywanie oszustw w transakcjach

**Grupa:** L3 · **Zespół:** Z8 · **Repozytorium:** https://github.com/maciejlewandowskii/financial-fraud-detection
Sztuczna inteligencja — laboratorium · Collegium Da Vinci · rok akad. 2026/2027

## 1. Problem *(rubryka A)*

Sklepy internetowe i banki przetwarzają ogromne ilości płatności kartą, a część z nich to oszustwa, np. zakupy kradzioną kartą. Nie da się sprawdzić każdej transakcji ręcznie, a pomyłka w każdą stronę kosztuje: przepuszczone oszustwo to strata pieniędzy, a zablokowana uczciwa płatność to zdenerwowany klient. Nasz program dostaje dane jednej transakcji (kwota, czas, typ produktu, dane karty, adres, domena e-mail, urządzenie i sporo anonimowych kolumn z datasetu). Zwraca prawdopodobieństwo, że to oszustwo, oraz decyzję: podejrzana albo w porządku. Całość działa jako aplikacja desktopowa, w której można przeglądać transakcje z datasetu albo wpisać własną i od razu zobaczyć, co na to model.

## 2. Dlaczego SI i dlaczego ta technika *(B · 7 pkt)*

Zwykły algorytm odpada, bo reguł jest za dużo i ciągle się zmieniają. Oszuści cały czas wymyślają nowe sposoby, więc ręcznie napisane warunki w stylu "kwota powyżej X i nowa karta" szybko przestałyby działać. Do tego dataset ma ponad 430 kolumn, w większości zanonimizowanych (V1-V339, C1-C14 itd.). Nie wiemy nawet, co te kolumny oznaczają, więc tym bardziej nie da się na ich podstawie pisać reguł. Model uczy się tych zależności sam z oznaczonych przykładów, a jak oszuści zmienią metody, wystarczy go wytrenować jeszcze raz na nowszych danych.

Wybraliśmy sieć neuronową (MLP), bo dobrze radzi sobie z dużą liczbą cech naraz i sama znajduje nieliniowe zależności między nimi. Kolumny tekstowe z tysiącami różnych wartości (np. numer karty card1, domeny e-mail) zamieniamy na embeddingi, czyli krótkie wektory liczb, których sieć uczy się razem z resztą. Gdybyśmy wybrali system ekspertowy, musielibyśmy sami wymyślić reguły, a przy zanonimizowanych kolumnach nie mamy z czego ich wyprowadzić.

## 3. Dane *(B · 8 pkt)*

Korzystamy z datasetu IEEE-CIS Fraud Detection z Kaggle (https://www.kaggle.com/c/ieee-fraud-detection). To prawdziwe transakcje e-commerce udostępnione przez firmę Vesta. Żeby je pobrać, trzeba mieć konto na Kaggle i zaakceptować zasady konkursu.

To nie był nasz pierwszy wybór. Na początku wzięliśmy zbiór Financial Transactions Dataset for Fraud Detection, też z Kaggle, w którym oszustwa stanowią 3,58% transakcji. Pierwszy model wytrenowany na nim osiągnął AUC-PR 4,39%, czyli tylko ok. 1,23 razy lepiej niż zgadywanie. Dalszy trening nic nie dawał: już pierwsza epoka dawała takie same wyniki jak kolejne, więc nie był to problem przetrenowania. Po prostu w tych danych było za mało informacji, żeby dało się odróżnić oszustwa od zwykłych transakcji. W dyskusjach na Kaggle przy tym zbiorze też pisano, że to praktycznie górna granica. Dlatego przeszliśmy na IEEE-CIS, który ma dużo więcej informacji o każdej transakcji (ponad 430 kolumn). Na nim nasz model osiąga AUC-PR 0,79, czyli ponad 22 razy lepiej niż zgadywanie (punkt 5).

Cały zbiór to ok. 1,35 GB plików CSV po rozpakowaniu (do pobrania jest archiwum ZIP, ok. 118 MB). Używamy z niego dwóch plików:
- `train_transaction.csv`: 590 540 transakcji, 394 kolumny, w tym etykieta `isFraud`,
- `train_identity.csv`: dodatkowe informacje o urządzeniu i przeglądarce, ale tylko dla części transakcji.

Pliki testowe z Kaggle pominęliśmy, bo nie mają etykiet. Oszustw jest tylko ok. 3,5%, więc dane są mocno niezbalansowane, co miało wpływ na wybór metryki (punkt 5). W repo jest też `sample/ieee-sample.csv` z 12 wierszami, żeby było widać, jak wyglądają dane.

Co robimy z danymi przed treningiem:
1. Łączymy oba pliki po `TransactionID`. Jeśli transakcja nie ma danych o urządzeniu, te pola zostają puste.
2. Dzielimy kolumny na liczbowe (382) i kategoryczne (49, np. ProductCD, card1-card6, domeny e-mail, M1-M9, DeviceInfo).
3. Dorabiamy 17 cech o "kliencie". Z analiz z konkursu wiadomo, że D1 to mniej więcej liczba dni od pierwszego użycia karty, więc dzień transakcji minus D1 jest stały dla danej karty. Razem z card1 i addr1 daje to coś w rodzaju identyfikatora klienta. Dla każdego klienta liczymy np. ile ma transakcji, jaka jest jego średnia kwota i czy obecna kwota mocno od niej odbiega.
4. Liczby przepuszczamy przez logarytm, bo kwoty mają bardzo długi ogon: większość to kilkadziesiąt dolarów, ale zdarzają się tysiące. Potem je standaryzujemy. Braki zamieniamy na 0 i dodajemy flagi mówiące, że wartości brakowało.
5. Z czasu transakcji wyciągamy porę dnia i dzień tygodnia, a z kwoty grosze (część po przecinku).
6. Dla kolumn kategorycznych robimy słownik wartości, które wystąpiły co najmniej 10 razy. Rzadsze wartości trafiają do wspólnej grupy "nieznana".

Dane dzielimy losowo: 80% trening, 10% walidacja, 10% test (472 432 / 59 054 / 59 054 transakcji), z takim samym odsetkiem oszustw w każdej części. Seed to 42, więc podział za każdym razem wychodzi taki sam.

Test nie widzi danych treningowych, bo to osobne wiersze. Wszystko, co w przygotowaniu danych się "uczy" (średnie do standaryzacji, słowniki kategorii), liczymy tylko na części treningowej. Na walidacji dobieramy próg decyzji, a test uruchamiamy raz, na samym końcu. Jest jedno zastrzeżenie: cechy klienta z punktu 3 liczymy na całym zbiorze. Nie używają etykiet, więc nie zdradzają odpowiedzi, ale ta sama karta może być i w treningu, i w teście (więcej w punkcie 7).

## 4. Rozwiązanie *(A · 10 pkt, B · 5 pkt)*

### Jak to działa

dane CSV → przygotowanie cech → trening sieci → zapisany model (`artifacts/`) → aplikacja

1. Program treningowy (`fraud-train`) wczytuje dane, dzieli je na części i przygotowuje cechy.
2. Sieć trenuje się przez 40 epok. Po każdej epoce w terminalu widać wyniki na walidacji.
3. Na walidacji wybieramy próg, od którego transakcja jest uznawana za oszustwo.
4. Na teście liczymy końcowe wyniki i zapisujemy wszystko do folderu `artifacts/`: wagi modelu, ustawienia przygotowania danych i raport z wynikami.
5. Aplikacja (Tauri) wczytuje ten model i ocenia transakcje wybrane z tabeli albo wpisane ręcznie.

Całość jest napisana w Ruście. Sieć jest zrobiona w bibliotece burn i liczy się na CPU, a interfejs to zwykły HTML i JavaScript.

### Model

Sieć to MLP z trzema warstwami ukrytymi (512, 256 i 128 neuronów, aktywacja ReLU). Na wejście idą przygotowane cechy liczbowe i embeddingi kolumn kategorycznych: każda wartość kategorii dostaje swój wektor 8 liczb. Na końcu jest jeden neuron. Jego wynik przepuszczamy przez sigmoidę, żeby dostać prawdopodobieństwo oszustwa:

```
p = 1 / (1 + e^(-z))
```

gdzie z to surowy wynik ostatniego neuronu.

Sieć uczy się, minimalizując entropię krzyżową (binary cross-entropy):

```
L = -[ y * ln(p) + (1 - y) * ln(1 - p) ]
```

y = 1 dla oszustwa, 0 dla zwykłej transakcji. Kara jest tym większa, im pewniej model się myli. Jeśli oszustwu dał 1% szans, strata wynosi ok. 4,6, a przy 50% tylko ok. 0,7. Gradient tej straty względem z wychodzi po prostu p - y, czyli dokładnie o tyle, o ile model się pomylił. Wagi poprawiamy spadkiem gradientu (optymalizator AdamW), a learning rate stopniowo zmniejszamy w trakcie treningu. Żeby sieć nie nauczyła się danych na pamięć, używamy dropoutu, czyli podczas treningu losowo wyłączamy część neuronów.

### Próg decyzji

Sieć zwraca tylko prawdopodobieństwo, a aplikacja musi powiedzieć "tak" albo "nie". Próg wybieramy na walidacji tak, żeby koszt błędów był jak najmniejszy:

```
koszt = 10 * fałszywe alarmy + 1 * przepuszczone oszustwa
```

Założyliśmy, że zablokowanie uczciwego klienta jest 10 razy gorsze niż przepuszczenie oszustwa. Dlatego model zgłasza oszustwo dopiero wtedy, gdy jest go naprawdę pewny. U nas próg wyszedł 0,945, czyli transakcja jest oznaczana jako oszustwo dopiero przy ponad 94% szans. Wagę fałszywego alarmu można zmienić parametrem `--fp-cost`.

### Najważniejsze parametry

| Parametr | Wartość |
|---|---|
| Warstwy ukryte | 512, 256, 128 |
| Rozmiar embeddingu | 8 |
| Dropout | 0,4 (na wejściu 0,1) |
| Epoki | 40 |
| Batch size | 4096 |
| Learning rate | 0,001, spada do 0,00001 |
| Weight decay | 0,01 |
| Podział danych | 80 / 10 / 10 |
| Seed | 42 |
| Koszt fałszywego alarmu | 10 |

## 5. Ewaluacja *(B · 8 pkt)*

Główna metryka to AUC-PR, czyli pole pod krzywą precision-recall. Accuracy się tu nie nadaje: przy 3,5% oszustw model, który zawsze mówi "wszystko w porządku", ma 96,5% accuracy i nie łapie ani jednego oszustwa. AUC-PR sprawdza, jak dobrze model wyłapuje właśnie tę rzadką klasę. Dodatkowo podajemy AUROC oraz precision i recall przy wybranym progu.

Próg sukcesu z karty tematu: AUC-PR ≥ 0,75 na zbiorze testowym.

Punkt odniesienia to model losowy. Jego AUC-PR jest równe odsetkowi oszustw, czyli ok. 0,035, a AUROC wynosi 0,5. Przy precision i recall porównujemy się z modelem, który zawsze mówi "w porządku" i nie łapie żadnego oszustwa.

| Przypadek | Punkt odniesienia | Nasze rozwiązanie |
|---|---|---|
| Test (59 054 transakcji, 2067 oszustw): AUC-PR | 0,035 | **0,793** |
| Test: AUROC | 0,5 | 0,958 |
| Test: precision / recall przy progu | - / 0% | 98,4% / 45,4% |
| Test: złapane oszustwa / fałszywe alarmy | 0 / 0 | 938 / 15 |
| Walidacja (59 054 transakcji): AUC-PR | 0,035 | 0,779 |
| Wybrany próg | - | 0,945 |

Wniosek: na teście model ma AUC-PR 0,79, czyli ponad 22 razy więcej niż model losowy, więc spełnia kryterium z karty tematu (≥ 0,75). Wynik na walidacji (0,78) jest bardzo zbliżony, więc nie jest to przypadek jednego konkretnego zbioru. Przy wybranym progu model łapie 938 z 2067 oszustw (45%) i daje tylko 15 fałszywych alarmów na prawie 57 tys. uczciwych transakcji, czyli zgodnie z założeniem z punktu 4 jest bardzo ostrożny.

## 6. Wymagania niefunkcjonalne *(A · 5 pkt)*

Czas działania (CPU):
- trening na pełnych danych (40 epok) trwał ok. 15-20 minut na MacBooku Pro z procesorem M1 Pro. Program sam wypisuje czas na końcu,
- żeby szybko sprawdzić, czy wszystko działa, można puścić trening na kawałku danych: `--limit 50000 --epochs 5`,
- ocena jednej transakcji w aplikacji jest praktycznie natychmiastowa,
- trening trzyma całą tabelę w pamięci, więc potrzeba kilku GB RAM.

Złe albo niepełne dane:
- jeśli brakuje pliku z danymi, wymaganej kolumny albo etykieta ma dziwną wartość, trening kończy się komunikatem, co jest nie tak (która kolumna, jaka wartość),
- `train_identity.csv` jest opcjonalny. Bez niego model uczy się tylko na transakcjach,
- puste pola to w tym datasecie norma, więc model je obsługuje: brakująca liczba jest traktowana jak średnia i oznaczana flagą, a brakująca kategoria ma osobną wartość,
- nieznana kategoria (np. domena e-mail, której nie było w treningu) nie powoduje błędu, tylko trafia do grupy "nieznana",
- aplikacja bez wytrenowanego modelu i tak się uruchamia i pokazuje komunikat, jaką komendą go wytrenować.

Powtarzalność: seed 42 jest ustawiony na sztywno w kodzie. Ustala podział danych, tasowanie i startowe wagi sieci, więc ponowne uruchomienie powinno dać praktycznie ten sam wynik. Nie możemy zagwarantować identycznego wyniku co do ostatniej cyfry, bo dane ładujemy w 4 wątkach i kolejność batchy może się minimalnie różnić. Trening na pełnych danych zrobiliśmy tylko raz, więc nie porównywaliśmy wyników dwóch uruchomień. Żeby to sprawdzić, wystarczy puścić trening drugi raz i porównać pliki `report.json`.

## 7. Ograniczenia *(E · 4 pkt)*

- **Losowy podział zamiast czasowego.** Dane obejmują ok. pół roku, więc model uczy się też na transakcjach "z przyszłości" względem części testu. Ta sama karta może też trafić i do treningu, i do testu. Przez to wynik na teście jest pewnie trochę lepszy, niż byłby w prawdziwym użyciu. Lepiej byłoby trenować na pierwszych miesiącach, a testować na ostatnim.
- **Nowi klienci i rzadkie dane.** Najgorzej wypadają nowe karty, których nie było w danych (cechy klienta są wtedy puste), transakcje bez danych o urządzeniu i rzadkie kategorie.
- **Ostrożny próg.** Przez wysoki koszt fałszywego alarmu sporo oszustw przechodzi: na teście model przepuścił 1129 z 2067 (55%). Bank, który bardziej boi się strat niż niezadowolonych klientów, powinien zmniejszyć `--fp-cost`.
- **Model z ostatniej epoki.** Zapisujemy model po ostatniej epoce, a nie ten, który miał najlepszy wynik na walidacji.
- **Brak wyjaśnień.** Model nie tłumaczy, dlaczego uznał transakcję za podejrzaną, a przy zanonimizowanych kolumnach trudno to nawet sprawdzić ręcznie.
- **Starzenie się modelu.** Z czasem trzeba go trenować od nowa. Działa też tylko na danych w formacie IEEE-CIS.
- **Przenoszenie na inny komputer.** `report.json` zapamiętuje pełną ścieżkę do danych, więc po skopiowaniu modelu na inny komputer przeglądarka transakcji w aplikacji ich nie znajdzie. Samo sprawdzanie transakcji działa.

Co dalej: podział po czasie, zapisywanie najlepszego modelu z walidacji, porównanie z LightGBM/XGBoost (na nich opierały się najlepsze rozwiązania tego konkursu) i wyjaśnianie decyzji modelu, np. przez SHAP.

## 8. Zastosowania w praktyce *(E · 4 pkt)*

- **Visa i Mastercard.** Przy każdej płatności kartą model w czasie rzeczywistym ocenia, jak ryzykowna jest transakcja, a bank bierze tę ocenę pod uwagę przy akceptacji (Visa Advanced Authorization, Mastercard Decision Intelligence).
- **Stripe i PayPal.** Modele uczone na płatnościach z wielu sklepów naraz (np. Stripe Radar) blokują podejrzane płatności albo wymuszają dodatkową weryfikację, np. 3-D Secure.
- **Vesta.** Firma, od której pochodzi nasz dataset, robi dokładnie to dla sklepów internetowych: ocenia ryzyko każdej transakcji i gwarantuje pokrycie strat z oszustw.

## 9. Zespół *(D · 8 pkt)*

| Osoba | Za co odpowiadała |
|---|---|
| Maciej Lewandowski | model: przygotowanie danych i cech, sieć neuronowa, trening, ewaluacja |
| Viachaslau Lukashonak | frontend: projekt i wykonanie aplikacji desktopowej (Tauri) i interfejsu użytkownika |
| Marcel Krzysztoń | karta tematu, dokumentacja PL i EN, prezentacja |

W karcie tematu dane i implementację mieli wspólnie Maciej i Viachaslau. W praktyce podzielili się tak, że Maciej zajął się modelem, a Viachaslau frontendem. Raporty ze sprintów robił każdy z nas. Raporty ze sprintów 7 i 8 są w folderze `sprints/`.

## 10. Uruchomienie *(C · 4 pkt)*

Potrzebne:
- Rust, najnowsza stabilna wersja, minimum 1.88 (https://rustup.rs),
- Node.js z npm,
- na Windowsie Microsoft C++ Build Tools (pozostałe wymagania Tauri: https://v2.tauri.app/start/prerequisites/),
- kilka GB wolnego RAM-u na trening,
- do pobrania danych: Python z pakietem `kaggle` i token API w `~/.kaggle/kaggle.json`. Można też pobrać dane ręcznie ze strony konkursu przyciskiem "Download All".

```sh
# 1. Repozytorium
git clone https://github.com/maciejlewandowskii/financial-fraud-detection.git
cd financial-fraud-detection

# 2. Dane (do folderu data/ieee)
pip install kaggle
kaggle competitions download -c ieee-fraud-detection -p data/ieee
unzip data/ieee/ieee-fraud-detection.zip -d data/ieee
# na Windowsie zamiast unzip: Expand-Archive data/ieee/ieee-fraud-detection.zip data/ieee

# 3. Trening
cargo run -p fraud-train --release
# szybka próba: cargo run -p fraud-train --release -- --limit 50000 --epochs 5

# 4. Aplikacja
cd app
npm install
npm run tauri dev
```

Gdzie są wyniki:
- w terminalu po treningu: AUC-PR, AUROC, precision, recall i wybrany próg,
- w folderze `artifacts/`: model i `report.json` z wynikami,
- w aplikacji: AUC-PR modelu widać w nagłówku, a po kliknięciu transakcji w tabeli (albo "Losuj") model pokazuje swoją ocenę obok prawdziwej etykiety.

Opcje treningu: `--data` (folder z danymi, domyślnie `data/ieee`), `--out` (gdzie zapisać model, domyślnie `artifacts`), `--epochs` (domyślnie 40), `--limit` (tylko pierwsze N transakcji), `--fp-cost` (domyślnie 10). Aplikacja może wczytać model z innego folderu przez zmienną środowiskową `FRAUD_MODEL_DIR`.

## 11. Narzędzia AI

Maciej korzystał z AI przy części modelowej:
- jako pomocy przy przeglądzie kodu: AI porównywało nasz kod z przykładowymi projektami z GitHuba i wskazywało błędy,
- przy analizie, dlaczego pierwszy wybrany zbiór danych (Financial Transactions Dataset for Fraud Detection z Kaggle) słabo nadawał się do projektu. Po tej analizie przeszliśmy na IEEE-CIS,
- przy normalizacji cech: kolumn jest kilkaset, więc ręczne dobieranie przekształceń dla każdej z nich byłoby bardzo pracochłonne.

Viachaslau zaprojektował i zbudował interfejs w Tauri w większości samodzielnie. AI pomagało mu tylko przy drobiazgach:
- przy małych poprawkach w kodzie,
- przy przygotowaniu przykładowych danych do sprawdzania wyglądu i działania UI,
- przy tymczasowych komendach-atrapach (mockach), które później zastąpiła prawdziwa logika z modelem.

Przy pisaniu dokumentacji korzystaliśmy z Claude (Claude Code). Pomógł przeanalizować kod i przygotować pierwszą wersję tekstu, którą potem poprawiliśmy i uzupełniliśmy o wyniki.
