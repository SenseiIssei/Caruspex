# Bezpieczeństwo i aktualizacje

Ta strona zbiera informacje o bezpieczeństwie Caruspex: do czego służy program, jakie ryzyka warto znać, jak
działają aktualizacje i jak go usunąć. Odpowiada informacjom dla użytkowników, których wymaga unijny akt o
cyberodporności (Cyber Resilience Act, załącznik II), w zakresie, w jakim dotyczą wolnego, niekomercyjnego programu.

## Program

- **Nazwa**: Caruspex, aplikacja `caruspex` i narzędzie wiersza poleceń `caruspex_cli`, dla Windows, Linuksa i
  macOS. Wersja jest w nazwie każdego pobrania i na liście zmian każdego wydania.
- **Wydawca**: projekt Caruspex na [GitHubie](https://github.com/SenseiIssei/Caruspex), jako wolne oprogramowanie na
  licencji MIT. Pytania i błędy trafiają do [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Przeznaczenie**: odczyt i objaśnianie danych diagnostycznych samochodów, motocykli i ciężarówek przez własny
  adapter, na własnym komputerze. Bez konta, bez chmury i bez śledzenia ([Prywatność](privacy.md)).
- **Cechy bezpieczeństwa**: odczyt nigdy niczego nie zmienia w samochodzie, a wszystko, co każe samochodowi coś
  zrobić, najpierw pyta. Aktualizacja jest instalowana tylko wtedy, gdy jej podpis pasuje do klucza wbudowanego w
  aplikację. Caruspex nie otwiera portów sieciowych i nie uruchamia kodu z plików ani z internetu.

## Zgłaszanie podatności {#report}

Problemy bezpieczeństwa prosimy zgłaszać prywatnie przez
[formularz nowego zgłoszenia bezpieczeństwa](https://github.com/SenseiIssei/Caruspex/security/advisories/new), a nie
w publicznym issue. [Polityka bezpieczeństwa](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) opisuje,
co dzieje się dalej. Naprawione podatności są publikowane jako
[komunikaty bezpieczeństwa](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Ryzyka, które warto znać {#risks}

- Pobieraj Caruspex tylko ze [strony wydań](https://github.com/SenseiIssei/Caruspex/releases). Kopie z innych miejsc
  mogą być zmienione.
- Otwieraj nagrania, pakiety PID i modele 3D tylko od osób, którym ufasz. Nie mogą uruchomić kodu, ale uszkodzony
  plik może spowolnić lub zamknąć Caruspex.
- Kasuj kody błędów, uruchamiaj testy elementów wykonawczych i ustawienia podstawowe tylko przy stojącym samochodzie,
  w bezpiecznym miejscu.
- Wiele adapterów Bluetooth i Wi-Fi nie ma PIN-u albo ma otwartą sieć: każdy w pobliżu może się połączyć. Po pracy
  wyjmij adapter.
- Kable ENET, DoIP i adaptery Wi-Fi niczego nie szyfrują: używaj ich bezpośrednio albo tylko we własnej sieci.
- Sterowniki J2534 to programy ich producentów i działają z twoimi uprawnieniami. Instaluj je tylko od producenta.

## Aktualizacje i wsparcie {#support}

- Przy starcie Caruspex pyta GitHub, czy jest nowsza wersja, i ją pokazuje. Instalacja to jedno kliknięcie, a
  aplikacja instaluje tylko pakiet z pasującym podpisem. AppImage i aplikacja na macOS aktualizują się same; pakiet
  .deb i aplikacja na Androida dostają stronę pobierania.
- Wyszukiwanie i automatyczna instalacja to przełączniki w ustawieniach, w sekcji **Okno i język**. Bez
  wyszukiwania zaglądaj czasem na stronę wydań.
- Poprawki bezpieczeństwa trafiają tylko do najnowszego wydania. Starsze wersje nie dostają osobnych poprawek, więc
  aktualizuj Caruspex.
- Caruspex jest rozwijany w wolnym czasie i nie ma stałej daty końca. Jeśli rozwój się skończy, repozytorium i
  ostatnie wydanie zapowiedzą to z wyprzedzeniem.

## Sprawdzanie pobranego pliku {#verify}

Każde wydanie ma plik `SHA256SUMS` z sumą kontrolną każdego pliku oraz podpisany przez GitHub zapis tego, jak pliki
zostały zbudowane. Z plikami w jednym folderze:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Bez tych narzędzi `Get-FileHash <plik>` w PowerShell (Windows) albo `shasum -a 256 <plik>` (macOS) pokazuje sumę do
porównania z wierszem w `SHA256SUMS`.

## Usuwanie Caruspex i własnych danych {#remove}

1. Odinstaluj program: w Windows w Ustawieniach, Aplikacje; w macOS przeciągnij Caruspex z folderu Aplikacje; w
   Linuksie usuń AppImage albo użyj `sudo apt remove caruspex`.
2. Usuń dane: przycisk **Otwórz miejsce przechowywania** w ustawieniach pokazuje foldery z ustawieniami, garażem i
   raportami awarii. Usuń je. Nagrania i raporty zapisane samodzielnie są tam, gdzie je zapisano.

## Dla programistów i firm {#integrators}

- Każde wydanie ma dla każdej platformy wykaz składników oprogramowania
  `Caruspex-<version>-sbom-<platform>.cdx.json` (CycloneDX) z każdą biblioteką i jej wersją.
- Ocena ryzyka, lista wymagań i zakres stosowania Cyber Resilience Act dla Caruspex są w
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex nie jest wprowadzany do obrotu jako produkt w rozumieniu Cyber Resilience Act, dlatego nie ma deklaracji
  zgodności UE ani oznakowania CE.
