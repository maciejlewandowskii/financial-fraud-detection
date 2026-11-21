---
share_link: https://share.note.sx/mdxqmhwe#9dalkcKZZG9oTSGt+Dqi/zjv8lvS1SslMGr78DAE6fY
share_updated: 2026-09-27T09:38:22+02:00
---
# Karta zgłoszenia tematu projektu

**Grupa:** L3 (niestacjonarna) **Data zgłoszenia:** 5 października 2026

---

## 1. Skład zespołu

|Imię i nazwisko|Nr albumu|Rola w zespole|
|---|---|---|
|Maciej Lewandowski|31831|dane, implementacja, ewaluacja|
|Marcel Krzysztoń|31889|dokumentacja, prezentacja|
|Viachaslau Lukashonak|30597|dane, implementacja|

Rola to nie stanowisko, tylko deklaracja, za co ta osoba odpowiada: dane, implementacja, ewaluacja, dokumentacja, koordynacja.

## 2. Tytuł projektu

**Po polsku: Wykrywanie oszustw w transakcjach**

**Po angielsku: Transactions fraud detection**

## 3. Problem

System do wykrywania oszustw i prób wyłudzeń w transakcjach. System ma być dostępny zarówno dla dużych firm i osób fizycznych.

## 4. Dlaczego zwykły algorytm nie wystarczy

_Wskażcie jeden z trzech powodów omawianych na zajęciach i uzasadnijcie:_

- [ ] nie umiemy zapisać reguły krok po kroku
- [ ] przepis istnieje, ale jest obliczeniowo za drogi
- [x] reguł jest za dużo i zmieniają się w czasie

Uzasadnienie, 2–3 zdania: Oszustwa to "dziedzina", która ciągle się zmienia. Praktycznie codziennie wymyślane są nowe sposoby kradzieży w sieci. Z tego też powodu niemożliwym jest stworzenie programu o stałych parametrach. 

---

> Punkty poniżej wypełniacie do 3.10.

---

## 5. Technologia SI

- [x] sieć neuronowa
- [ ] system ekspertowy / wnioskowanie regułowe
- [ ] algorytm genetyczny
- [ ] klasyczne uczenie maszynowe
- [ ] inne:

Dlaczego akurat ta technologia pasuje do tego problemu, 2–3 zdania: Zbiór IEEE-CIS ma ponad 430 kolumn, w większości zanonimizowanych (V1–V339, C, D, M), z dużą liczbą braków i kategoriami o bardzo wielu wartościach (np. numer karty, domena e-mail), więc ręczne wyprowadzenie z nich reguł nie jest możliwe. Sieć neuronowa (perceptron wielowarstwowy z embeddingami dla cech kategorycznych) sama uczy się nieliniowych zależności między cechami. Gdy pojawią się nowe schematy oszustw, wystarczy ponownie wytrenować ją na świeżych danych zamiast przepisywać reguły (por. punkt 4).

## 6. Dane

|||
|---|---|
|Źródło|https://www.kaggle.com/competitions/ieee-fraud-detection?fbclid=IwY2xjawVMo1JleHRuA2FlbQIxMQBwZG9mA3NydGMGYXBwX2lkATAAAR4sxU8qFMqFMHSk1Ffs0wVBW4fAGD61Sp0GSqy_t8AimIOZd9wliT1-D4YrKw_aem_6PlxMj9-8-V9Ei-DXI7Vpw|
|Rozmiar i format|.csv - 1,35 GB po rozpakowaniu (archiwum .zip ok. 118 MB); do treningu używamy train_transaction.csv i train_identity.csv (590 540 transakcji)|
|Czy są już dostępne?|tak |
|Jeśli nie — plan pozyskania|n/a|

## 7. Stos technologiczny

Rust, Tauri

## 8. Kryterium sukcesu

AUC-PR (pole pod krzywą precision-recall) na wydzielonym zbiorze testowym (10% danych) ≥ 0,75. Dla porównania model losowy ma AUC-PR ok. 0,035, bo tyle wynosi odsetek oszustw w danych.

## 9. Zakres minimalny

System potrafiący wykryć i powiadomić o próbie oszustw. 

## 10. Zakres opcjonalny

Frontend umożliwiający płynne użytkowanie programu. 

## 11. Repozytorium

Link: https://github.com/maciejlewandowskii/financial-fraud-detection

## 12. Główne ryzyko i plan awaryjny

_Model będzie niedotrenowany. Trzeba będzie znaleźć korelację błędów w datasecie._

## 13. Podział pracy w czasie

|Etap|Kto|Szacowany czas|
|---|---|---|
|Dane i przygotowanie|Maciej Lewandowski, Viachaslau Lukashonak|10|
|Implementacja|Maciej Lewandowski, Viachaslau Lukashonak|10|
|Eksperymenty i ewaluacja|Maciej Lewandowski|5|
|Dokumentacja PL|Marcel Krzysztoń|3|
|Dokumentacja EN|Marcel Krzysztoń|3|
|Prezentacja|Marcel Krzysztoń|2|
|**Razem na osobę**||**28–30 h**|

---

## Decyzja prowadzącego

_Wypełnia prowadzący — nie edytujcie tej sekcji._

- [ ] zatwierdzony
- [ ] do poprawy

**Uwagi:**