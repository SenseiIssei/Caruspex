# Usterki

Strona **Pamięć usterek** odczytuje, co zapisały sterowniki, i wyjaśnia to prostymi słowami.

![Pamięć usterek](screenshot-faults.png)

## Co widzisz

- Kody **zapisane, oczekujące i stałe**. Przy kilku sterownikach każdy kod pokazuje, kto go zgłosił (silnik, skrzynia biegów).
- Około 9400 kodów standardowych (P, C, B, U) oraz kody producentów VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai i Kia, Subaru i innych. Kod P1 jest odczytywany tak, jak rozumie go rozpoznana marka.
- **Sygnalizator** przy każdym kodzie i pytanie, które każdy zadaje najpierw: czy mogę jechać dalej?
- Typowe przyczyny, rzeczy, które możesz sprawdzić sam, i przybliżona klasa kosztów.
- Gdzie jest dana część: auto w widoku przezroczystym albo [widok rentgenowski](xray.md).

## Wszystkie sterowniki

W trybie profesjonalnym strona **Wszystkie sterowniki** odpytuje każdy sterownik, nie tylko silnika: ABS, poduszki powietrzne, zestaw wskaźników, klimatyzację i inne. Każda usterka ma rodzaj błędu (zwarcie, przerwa, nieprawdopodobny sygnał), numer części i wersję oprogramowania. Starsze auta koncernu VW są odczytywane przez [kabel KKL](kkl.md).

## Co mam teraz zrobić?

Strona **Co robić?** sortuje wszystko, co Caruspex znalazł, na trzy kroki: zatrzymaj się od razu, wkrótce do warsztatu, może poczekać. Każdy punkt ma jedno proste zdanie, dlaczego, przybliżony zakres kosztów i notatkę do pokazania w warsztacie. OBD nie widzi, czy kontrolka silnika miga albo czy świeci czerwona lampka ostrzegawcza, więc pytają cię o to dwa pola; migająca kontrolka z wypadaniem zapłonów przesuwa punkt do "zatrzymaj się od razu".

![Co mam teraz zrobić](screenshot-todo.png)

## Kasowanie kodów

Kasowanie jest możliwe dla każdego sterownika osobno, z wyraźnym ostrzeżeniem, a Caruspex zaraz potem odczytuje pamięć ponownie.

> Kasowanie to nie naprawa. Resetuje też monitory gotowości: dopóki auto nie przejedzie kilku pełnych cykli jazdy, nie przejdzie badania spalin, a kod, którego przyczyna wciąż istnieje, wraca po krótkiej jeździe.

## Eksport

Listę można zapisać jako CSV, a raport z kontroli jako PDF.
