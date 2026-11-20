# Prywatność

Caruspex działa na twoim komputerze. Nie ma konta, chmury ani śledzenia.

## Co jest zapisywane

Wszystko zostaje na twoim komputerze:

| Co | Kiedy | Gdzie |
|---|---|---|
| Ustawienia | zawsze | folder ustawień twojego systemu (pokazuje go przycisk **Otwórz miejsce przechowywania** w ustawieniach) |
| Garaż: VIN, auto, kontrole, stany licznika, ostatnie działające połączenie | po podłączeniu auta | obok ustawień, w folderze danych Caruspex |
| Nagrania i pliki CSV | tylko gdy je uruchomisz | tam, gdzie je zapiszesz |
| Plik dziennika | tylko jeśli włączysz go w ustawieniach | ścieżka, którą wybierzesz |
| Raporty awarii | tylko jeśli Caruspex się zawiesi | folder `crash-reports` obok garażu |
| Raporty z pierwszego testu KKL | tylko gdy zapiszesz raport | folder `test-reports` w folderze ustawień; numer VIN i numer immobilizera są skracane, chyba że zaznaczysz pole |

Każdy z tych plików możesz w dowolnej chwili usunąć.

## Co jest wysyłane

Przy każdej instalacji, także po aktualizacji ze starszej wersji, Caruspex pyta raz, czy ma szukać aktualizacji. Dopiero po odpowiedzi "tak" pyta GitHub, czy jest nowsza wersja, przy starcie i co sześć godzin, dopóki działa. Nic o tobie ani o aucie nie jest przy tym wysyłane. Bez "tak" żadne zapytanie nie trafia do GitHuba. Możesz to zmienić w każdej chwili w ustawieniach, w sekcji **Okno i język**. Poza tym Caruspex nie łączy się z internetem. Komunikacja z adapterem Wi-Fi, kablem ENET czy autem DoIP zostaje w twojej sieci lokalnej.

Aktualizacji szukają tylko wydania opublikowane na GitHubie; kopia zbudowana samodzielnie nigdy tego nie robi. Gdy Caruspex szuka aktualizacji lub ją pobiera, GitHub widzi, jak przy każdym pobieraniu, adres IP komputera i wersję Caruspex; sam Caruspex nie wysyła ani nie zapisuje nic więcej, a nikt inny, także projekt, tego nie widzi. Obowiązuje przy tym oświadczenie o prywatności GitHuba. Aktualizacje są pobierane tylko z wydań repozytorium Caruspex. Przyciski takie jak **Zgłoś na GitHubie** tylko otwierają przeglądarkę; to, co tam opublikujesz, zależy od ciebie.

## Android

Do Androida 11 system pozwala aplikacji znaleźć adaptery Bluetooth LE tylko z uprawnieniem do lokalizacji. Caruspex prosi o nie wyłącznie z tego powodu i nigdy nie odczytuje twojej lokalizacji. Od Androida 12 prosi zamiast tego o uprawnienie Bluetooth, oznaczone jako nigdy nieużywane do lokalizacji.

## Raporty awarii tylko na życzenie

Jeśli Caruspex się zawiesi, zapisuje krótki raport na twoim komputerze: wersję, system, miejsce w kodzie i łańcuch wywołań. Numery VIN, bajty ruchu z autem i twój folder domowy są wycinane. Nic nie jest wysyłane. Caruspex pokazuje ci raport i oferuje dwa przyciski: **Zgłoś na GitHubie** otwiera w przeglądarce wstępnie wypełnione zgłoszenie, które czytasz i zmieniasz, zanim je opublikujesz, a **Nie zgłaszaj** odkłada je na bok.

## Udostępnianie nagrań bez danych osobowych

Nagrania z prawdziwych aut pomagają ulepszać Caruspex dla większej liczby aut. **Udostępnij nagranie** przygotowuje kopię i dokładnie pokazuje, co zostaje, a co znika:

- **Usuwane**: ostatnie cyfry numeru VIN (wszędzie w pliku), nazwa portu i adres Bluetooth lub sieciowy adaptera, godzina.
- **Nigdy w nagraniu**: twoja lokalizacja i dane użytkownika.
- **Zostaje**: zapytania i odpowiedzi, typ adaptera, protokół i data.

Plik zostaje na twoim komputerze, dopóki sam go nie zapiszesz i nie dołączysz do zgłoszenia na GitHubie. Nic nie jest wysyłane automatycznie.

## Program testów

[Formularz raportu z testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) pyta o auto, rok, silnik i adapter. Prosi tylko o skrócony VIN; nigdy nie publikuj pełnego numeru VIN, numeru rejestracyjnego ani swojego nazwiska.
