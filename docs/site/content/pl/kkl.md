# Kabel KKL (VAG 409.1)

Kabel KKL to kabel USB z układem szeregowym i driverem linii K-Line, bez ELM327 w środku. Caruspex sam steruje linią K-Line, bajt po bajcie. Tego potrzebują starsze VW, Audi, Seat i Skoda: ich protokół KWP1281 potwierdza każdy pojedynczy bajt, a tego nie potrafi żaden ELM327.

## Co działa

- Każdy sterownik VW, Audi, Seata i Skody z lat około 1990 do 2004: silnik, skrzynia biegów, ABS, poduszki powietrzne, zestaw wskaźników, komfort i inne.
- KWP1281 i KWP2000 (ISO 14230), rozpoznawane automatycznie po bajtach kluczowych po wybudzeniu.
- Pamięć usterek każdego sterownika, bloki pomiarowe i prawdziwe ustawienie podstawowe przepustnicy (blok pomiarowy 060 lub 098).
- Kanały adaptacji są celowo tylko do odczytu. Test wyjść uruchamia prawdziwe części i startuje dopiero po twoim potwierdzeniu.

Te strony są w **trybie profesjonalnym**: **Sterowniki przez KKL**, **Przepustnica** oraz **Adaptacja i testy wyjść**. Każda sama otwiera kabel: wybierz port szeregowy, sterownik, start.

## Jaki kabel?

Kable z oryginalnym FTDI FT232RL lub CP2102 są na linii K-Line znacznie bardziej niezawodne niż najtańsze klony. Do KWP1281 zalecany jest oryginalny FTDI. Kabel musi przepuszczać K-Line bez zmian: interfejsy "HEX-USB" z własnym firmware nie działają.

Auta sprzed około 1996 roku często nie mają gniazda OBD, tylko wtyczkę 2x2 w komorze silnika albo pod deską rozdzielczą. Przejściówka z 2x2 na gniazdo 16-pinowe jest tania.

## Sprawdzenie kabla

Narzędzie wiersza poleceń testuje kabel bez auta:

```
caruspex_cli kklcheck --port COM3
```

## Pierwszy test w aucie

Strona **Pierwszy test w aucie** w trybie profesjonalnym prowadzi przez pierwszą sesję z nowym kablem KKL. Jest przygotowana z myślą o Audi A2 i działa w każdym VW, Audi, Seacie lub Skodzie z K-Line. Wykonuje tylko odczyt: nic nie jest kasowane, bez testu wyjść, bez ustawienia podstawowego. Port otwiera się dopiero po starcie testu.

1. **Sprawdź kabel bez auta.** Podłącz kabel tylko do USB, wybierz jego port i naciśnij **Sprawdź kabel**. Caruspex odczytuje układ (FTDI, CH340, Prolific, CP210x), przy FTDI timer opóźnienia, i mierzy bity adresu 5 baud na linii break. Bez auta echo nie wraca i tak ma być.
2. **Ustaw timer opóźnienia na 1 ms** (kable FTDI w Windows): Menedżer urządzeń, Porty (COM i LPT), kliknij dwukrotnie USB Serial Port, Ustawienia portu, Zaawansowane, Latency Timer (msec) na 1, OK, potem raz odłącz i podłącz kabel. Przy domyślnych 16 ms odpowiedź na bajty klucza może nie zmieścić się w swoim oknie 50 ms. Caruspex odczytuje to ustawienie, ale nigdy go nie zmienia.
3. **W aucie:** zapłon włączony, silnik wyłączony, kabel mocno w gnieździe OBD i bezpośrednio przy komputerze (bez huba USB), żaden inny program diagnostyczny nie jest otwarty. Zaznacz listę i naciśnij **Uruchom pierwszy test**.
4. Test wybudza silnik (adres 01) i mierzy każdy bit adresu, bajt synchronizacji 0x55, bajty klucza i odpowiedź na nie. Potem odczytuje identyfikację, pamięć usterek i bloki pomiarowe 001 do 003, a następnie zestaw wskaźników 17, ABS 03, poduszki powietrzne 15, komfort 46 i centralną elektrykę 09. Sterownik, który nie odpowiada, może po prostu nie być zamontowany.
5. **Zapisz raport** zapisuje plik tekstowy i plik JSON w folderze `test-reports` w folderze ustawień Caruspex. **Otwórz raport z testu na GitHubie** wypełnia za ciebie formularz raportu. Numer VIN i numer immobilizera są skrócone, dopóki nie zaznaczysz pola.

Co oznaczają wyniki:

| Wynik | Co zrobić |
|---|---|
| Brak echa | Kabel nie słyszy sam siebie: wtyczka nie siedzi dobrze albo na pinie 16 brakuje 12 V (sprawdź bezpiecznik gniazda diagnostycznego). Bez auta to normalne. |
| Echo jest zniekształcone | Inne urządzenie w gnieździe, przełącznik kabla K+DCAN nie na K-Line albo uszkodzony kabel. |
| Adres wybudzenia jest taktowany zbyt nierówno | Bity odbiegają o ponad 20 ms od 200 ms: bez huba USB, zamknij inne programy, timer opóźnienia 1 ms. Caruspex próbuje jeszcze raz z wcześniejszymi wywołaniami break. |
| Kabel w porządku, ale brak odpowiedzi z auta | Czy zapłon jest włączony? Adres 01 to silnik. Auta od około 2005 roku diagnozuje się przez CAN. |
| Odpowiedź na bajty klucza przyszła za późno | Ustaw timer opóźnienia na 1 ms. |

## Typowe problemy

- **Brak odpowiedzi ze sterownika**: zapłon włączony? Wybudzenie z prędkością 5 bodów wymaga dokładnych czasów; huby USB i niektóre maszyny wirtualne je psują, podłącz kabel bezpośrednio.
- **Po chwili się rozłącza**: tanie kable gubią bajty. Wypróbuj inny kabel, zanim zaczniesz podejrzewać auto.
