# Bezpieczeństwo, znaki towarowe i licencje

## Bezpieczeństwo i odpowiedzialność

Caruspex to wolny projekt hobbystyczny dostarczany bez żadnej gwarancji, jak mówi [licencja MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Używasz go na własne ryzyko.

- Nigdy nie obsługuj go podczas jazdy. Konfiguruj go na postoju; pasażer może śledzić wartości na żywo.
- Kasowanie błędów, testy elementów wykonawczych i ustawienia podstawowe sprawiają, że samochód coś robi. Caruspex pyta przed każdym z nich; wykonuj je tylko na postoju w zabezpieczonym samochodzie i tylko w pojazdach, przy których wolno ci pracować.
- Caruspex nie zastępuje warsztatu. Kod błędu wskazuje objaw, nie zawsze przyczynę. Hamulce, poduszki powietrzne, układ kierowniczy i wszystko, co ważne dla bezpieczeństwa, należy powierzyć fachowcom.
- Kasowanie to nie naprawa. Skasowane błędy i wyzerowane testy gotowości mogą sprawić, że badanie spalin lub przegląd techniczny nie zostaną zaliczone.
- Caruspex nie zmienia przebiegu, immobilizera, kluczyków, układu oczyszczania spalin ani tachografu, a projekt nie przyjmuje zmian, które by to robiły. Manipulowanie licznikami przebiegu, układem oczyszczania spalin lub tachografami jest w wielu krajach zabronione.

## Znaki towarowe

Nazwy producentów pojazdów, modeli, adapterów, programów i norm (na przykład Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS czy ISTA) są znakami towarowymi ich właścicieli. Pojawiają się tylko po to, by powiedzieć, z jakimi samochodami, adapterami i interfejsami działa Caruspex. Caruspex jest niezależnym projektem i nie jest powiązany, polecany ani sponsorowany przez żadnego producenta pojazdów, producenta adapterów ani dostawcę oprogramowania. Aplikacja i to repozytorium nie używają logo tych firm.

## Licencja i źródła

Kod jest na licencji MIT, zobacz [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Biblioteki zawarte w programach i ich licencje są wymienione w [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); aplikacja pokazuje je w ustawieniach. Dane wbudowane w programy:

- Wartości producentów dla około 550 pojazdów, przekształcone z [OBDb](https://github.com/OBDb), jak OBDb na licencji CC BY-SA 4.0: oddzielone od kodu w [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) z licencją i listą źródeł ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Definicje sygnałów CAN z [opendbc](https://github.com/commaai/opendbc) (comma.ai) i [model3dbc](https://github.com/joshwardell/model3dbc), licencja MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Wartości akumulatora dla Hyundaia Ioniq i Kona, Kii e-Niro, Nissana Leaf i Renault Zoe według OBDb (CC BY-SA 4.0, jak te pakiety), [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3) i [evDash](https://github.com/nickn17/evDash) (oba na licencji MIT): [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- Kody silników z artykułów Wikipedii, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), z artykułem i wersją w każdym wierszu).
- Kody producenta z numeru VIN sprawdzone w [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (domena publiczna) i [Wikidata](https://www.wikidata.org) (CC0); kody, które ma tylko [lista WMI w Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), są na licencji CC BY-SA 4.0 i leżą osobno w [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi).
- Wymiary pojazdów z [Wikidanych](https://www.wikidata.org), CC0.
- Czcionki na licencji SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
