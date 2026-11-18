# Pierwsze kroki

## 1. Wypróbuj samochód demo

Nie potrzebujesz auta, żeby się rozejrzeć. Na stronie **Połącz** wybierz **Samochód demo**: jeździ, nagrzewa się, ma zapisanych kilka usterek i odpowiada jak prawdziwy ELM327. Działa z nim każda strona.

## 2. Wybierz język i tryb

Caruspex używa języka twojego systemu. W **Ustawieniach** możesz wybrać jeden z 18 języków oraz jasny lub ciemny wygląd; obie zmiany działają od następnego uruchomienia.

**Tryb początkujący** pokazuje strony na co dzień. **Tryb profesjonalny** dodaje narzędzia warsztatowe: strony KKL, adaptację, surowe polecenia adaptera i adresy sterowników.

## 3. Podłącz auto

1. Znajdź gniazdo diagnostyczne: [asystent połączenia](connect.md) pokazuje, gdzie jest w twoim aucie.
2. Włóż adapter i włącz zapłon. Silnik może pracować, ale nie musi.
3. Na stronie **Połącz** wybierz rodzaj połączenia albo pozwól Caruspex samemu przeszukać wszystkie porty.

Przy pierwszym uruchomieniu asystent połączenia otwiera się sam. Każdy krok można pominąć, a asystent jest później dostępny w ustawieniach i na stronie Połącz.

![Strona Połącz](screenshot-connect.png)

## 4. Pierwsze spojrzenie na auto

Po połączeniu Caruspex odczytuje VIN i mówi, jaki to samochód. Dobre miejsca na start:

- **Kontrola kondycji**: jeden przycisk, pół minuty, wynik od 0 do 100, a każdy utracony punkt jest wyjaśniony.
- **Co mam teraz zrobić?**: wszystko, co znaleziono, posortowane na trzy kroki.
- [Pamięć usterek](faults.md) i [wartości na żywo](live.md).

![Kontrola kondycji](screenshot-health.png)

## 5. Garaż i raporty

Każde auto jest rozpoznawane po numerze VIN i dostaje własny wpis w garażu, z historią kontroli i stanów licznika. Raport z kontroli można zapisać jako PDF dla warsztatu.
