# Kabel KKL (VAG 409.1)

Kabel KKL to kabel USB z układem szeregowym i driverem linii K-Line, bez ELM327 w środku. Caruspex sam steruje linią K-Line, bajt po bajcie. Tego potrzebują starsze VW, Audi, Seat i Skoda: ich protokół KWP1281 potwierdza każdy pojedynczy bajt, a tego nie potrafi żaden ELM327.

## Co działa

- Każdy sterownik VW, Audi, Seata i Skody z lat około 1990 do 2004: silnik, skrzynia biegów, ABS, poduszki powietrzne, zestaw wskaźników, komfort i inne.
- KWP1281 i KWP2000 (ISO 14230), rozpoznawane automatycznie po bajtach kluczowych po wybudzeniu.
- Pamięć usterek każdego sterownika, bloki pomiarowe i prawdziwe ustawienie podstawowe przepustnicy (blok pomiarowy 060 lub 098).
- Kanały adaptacji i testy wyjść są celowo tylko do odczytu.

Te strony są w **trybie profesjonalnym**: **Sterowniki przez KKL**, **Przepustnica** oraz **Adaptacja i testy wyjść**. Każda sama otwiera kabel: wybierz port szeregowy, sterownik, start.

## Jaki kabel?

Kable z oryginalnym FTDI FT232RL lub CP2102 są na linii K-Line znacznie bardziej niezawodne niż najtańsze klony. Do KWP1281 zalecany jest oryginalny FTDI. Kabel musi przepuszczać K-Line bez zmian: interfejsy "HEX-USB" z własnym firmware nie działają.

Auta sprzed około 1996 roku często nie mają gniazda OBD, tylko wtyczkę 2x2 w komorze silnika albo pod deską rozdzielczą. Przejściówka z 2x2 na gniazdo 16-pinowe jest tania.

## Sprawdzenie kabla

Narzędzie wiersza poleceń testuje kabel bez auta:

```
caruspex_cli kklcheck --port COM3
```

## Typowe problemy

- **Brak odpowiedzi ze sterownika**: zapłon włączony? Wybudzenie z prędkością 5 bodów wymaga dokładnych czasów; huby USB i niektóre maszyny wirtualne je psują, podłącz kabel bezpośrednio.
- **Po chwili się rozłącza**: tanie kable gubią bajty. Wypróbuj inny kabel, zanim zaczniesz podejrzewać auto.
