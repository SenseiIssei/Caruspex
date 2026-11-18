# Caruspex

Caruspex to darmowy program diagnostyczny do samochodów i motocykli. Działa na Windows, Linux i macOS, mówi w 18 językach i współpracuje z adapterami, które ludzie naprawdę mają: tanią wtyczką ELM327, OBDLinkiem, kablem KKL do starszych aut z koncernu Volkswagena, kablem ENET do BMW i profesjonalnymi interfejsami CAN.

![Przegląd ze wskaźnikami na żywo](screenshot-overview.png)

## Co potrafi

- Odczytuje i kasuje kody błędów, wyjaśnia je prostymi słowami i mówi, czy możesz jechać dalej.
- Pokazuje wartości na żywo: temperatury, prędkość, skład mieszanki, ciśnienie doładowania, napięcie akumulatora i wiele więcej, jako liczby i jako wykresy.
- Odpytuje każdy sterownik w aucie, nie tylko silnika, na tyle, na ile pozwala samochód i adapter.
- Sprawdza auto używane przed zakupem: czy VIN się zgadza, czy przebiegi są zgodne, czy krótko wcześniej skasowano błędy?
- Pokazuje auto jako szklany model, w którym usterki i wartości świecą tam, gdzie ich miejsce.
- Mówi, co zrobić dalej: zatrzymać się od razu, wkrótce jechać do warsztatu albo może poczekać.

## Od czego zacząć

1. [Zainstaluj Caruspex](install.md) na swoim komputerze.
2. Przejdź [pierwsze kroki](first-steps.md): wypróbuj samochód demo, potem podłącz własny.
3. Znajdź swój adapter w dziale [adaptery i interfejsy](interfaces.md).
4. Przeczytaj w dziale [prywatność](privacy.md), co Caruspex zapisuje i wysyła. W skrócie: nic nie opuszcza twojego komputera, chyba że sam to udostępnisz.

> Caruspex tylko odczytuje. Nigdy nie koduje sterowników, nigdy nie zmienia przebiegu i nigdy nie rusza immobilizera. Kasowanie kodów błędów to jedyna rzecz, którą zapisuje w aucie.

## Uczciwy stan

Caruspex jest zbudowany według specyfikacji i testowany na symulowanych sterownikach, udawanych bramkach i udawanych sterownikach urządzeń. Na prawdziwym sprzęcie sprawdzono do tej pory tylko OBDLinka z układem STN2232, bez samochodu. Jeśli wypróbujesz Caruspex na swoim aucie, wyślij proszę [raport z testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): każdy raport wydłuża listę przetestowanych aut.
