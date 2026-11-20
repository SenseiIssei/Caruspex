# Symulator tuningu

Strona **Symulator tuningu** (tryb profesjonalny, w sekcji Pomiar) to wirtualny BMW N55: rzędowa szóstka 3,0 litra z jedną turbosprężarką twin-scroll z F30 335i i F32/F36 435i, 306 KM i 400 Nm. Zmieniaj doładowanie, zapłon i lambdę w zakresach obrotów i zobacz, co na to model, na długo zanim cokolwiek dotknie auta.

> **Tylko symulacja.** Caruspex nigdy nie zapisuje map silnika i nigdy nie programuje sterownika. Nie zna żadnego formatu plików do flashowania, żadnej sumy kontrolnej ani adresu mapy i nie kopiuje żadnego komercyjnego tuningu. Prawdziwy tuning to sprawa sprawdzonych narzędzi i warsztatu, a zmiana wymaga homologacji (w Niemczech przez TÜV). Bez niej auto traci homologację.

![Symulator tuningu](screenshot-tune.png)

## Co możesz zmienić

- Przesunięcia w pięciu zakresach obrotów (1000 do 2000, 2000 do 3500, 3500 do 5000, 5000 do 6000, 6000 do 7000): doładowanie w bar, zapłon w stopniach, lambda.
- Paliwo (RON 95, 98, 100 lub 102), temperaturę zewnętrzną, ciśnienie atmosferyczne i skrzynię (automat 8HP45 albo sześciobiegowa manualna).
- **Seria** zeruje przesunięcia, **Resetuj** także warunki.

Strona pokazuje moment i moc seryjne obok zmienionych, każdą granicę, której dotyka zmiana, z jej powodem, oraz wartości wewnętrzne: doładowanie, powietrze doładowania, zapłon, zapas do stuków, temperaturę spalin, turbo, pompę wysokiego ciśnienia, średnie ciśnienie użyteczne.

## Jak liczy model

1. **Powietrze.** Cel doładowania ogranicza to, co turbo potrafi dać przy tych obrotach, jego spręż i jego przepływ. Sprężarka ogrzewa powietrze, intercooler odbiera część ciepła. Ciśnienie, temperatura i współczynnik napełnienia dają masę powietrza.
2. **Paliwo i zapłon.** Paliwo wynika z powietrza i lambdy. Zapłon to seryjny kąt plus twoje przesunięcie, ale nigdy wcześniej, niż pozwala granica spalania stukowego: regulacja spalania stukowego silnika by go cofnęła. Granica przesuwa się z liczbą oktanową, temperaturą powietrza, doładowaniem i lambdą.
3. **Moc.** Energia paliwa razy sprawność, która spada z odległością od najlepszego kąta zapłonu, minus tarcie, daje moment i moc.
4. **Granice.** Turbo, temperatura spalin, pompa wysokiego ciśnienia, wtryskiwacze, przedwczesny zapłon przy niskich obrotach, skrzynia, powietrze doładowania i mieszanka dają każde ostrzeżenie prostymi słowami.

Cała krzywa (25 punktów od 1000 do 7000 obr./min) zajmuje w wersji wydania około 0,015 ms.

## Opublikowane dane

| Wartość | Liczba | Źródło |
|---|---|---|
| Pojemność, średnica, skok, stopień sprężania | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Moment obrotowy | 400 Nm od 1200 do 5000 obr./min | Wikipedia "BMW N55", dane BMW dla 435i |
| Moc | 225 kW (306 KM) przy 5700 do 6000 obr./min | Wikipedia (5700 do 5800), BMW (5800 do 6000) |
| Obroty maksymalne | 7000 obr./min | Wikipedia "BMW N55" |
| Nominalny moment wejściowy 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Założenia

Nikt nie publikuje na wolnej licencji wewnętrznych map N55. Te wartości to moje założenia; każde jest uzasadnione w [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- **Seryjne doładowanie** jest wyliczone wstecz modelem z opublikowanej krzywej (około 0,7 do 0,8 bar do 5000 obr./min, wyżej mniej). To, że krzywa seryjna się zgadza, pokazuje więc spójność, a nie prawdę.
- Współczynnik napełnienia 0,86 do 0,97, sprawność indykowana 0,39, tarcie kwadratowe względem obrotów.
- Zapłon ograniczony spalaniem stukowym od 3 do 15 stopni przed górnym martwym punktem z 1,5 stopnia zapasu; przesuwa się o 1 stopień na liczbę oktanową, 0,1 stopnia na kelwin powietrza, 6 stopni na bar i 2,5 stopnia na 0,1 lambdy.
- Turbo: spręż do 2,55, przepływ skorygowany do 0,32 kg/s, sprawność 0,72. Skuteczność intercoolera 0,85, mniej przy dużym przepływie.
- Granica temperatury spalin 980 °C, pompa wysokiego ciśnienia 0,027 kg/s, okno wtrysku 40 % cyklu, ryzyko przedwczesnego zapłonu powyżej 18 bar średniego ciśnienia użytecznego poniżej 2500 obr./min, skrzynia manualna około 450 Nm.
- Paliwo E5: stosunek powietrza do paliwa 14,4, 42,4 MJ/kg.

## Jak dokładne to jest?

- Seria wobec opublikowanej krzywej: 400 Nm z dokładnością 1 % od 1250 do 5000 obr./min, 306 KM przy 6000 obr./min i 304 KM przy 5750 obr./min. Testy dopuszczają 3 %.
- Model nie był jeszcze sprawdzony na prawdziwym N55. Każda liczba to szacunek: około ±8 % dla momentu i mocy, ±0,1 bar doładowania, ±10 K powietrza doładowania, ±3 stopnie zapasu do stuków, ±60 K temperatury spalin.

## Porównanie z twoim autem

1. Połącz się, otwórz **Wartości na żywo** z obrotami, obciążeniem, ciśnieniem w kolektorze, temperaturą powietrza zasysanego, kątem zapłonu, lambdą i masą powietrza, i uruchom nagrywanie.
2. Raz przyspiesz z gazem w podłodze przez cały zakres obrotów, tylko tam, gdzie to dozwolone i bezpieczne.
3. Otwórz nagranie na stronie symulatora. Caruspex bierze punkty przy pełnym obciążeniu, grupuje je w zakresy po 500 obr./min i pokazuje zmierzone obok symulowanych: ciśnienie w kolektorze i powietrze doładowania z twoimi mapami, masę powietrza i zapłon z zmierzonym ciśnieniem i temperaturą.

OBD podaje ciśnienie w kolektorze tylko do 255 kPa, a którego czujnika sterownik używa dla temperatury zasysania, nie jest udokumentowane. **Pokaż przykład** wypełnia tabelę wymyślonymi wartościami, żeby było widać, jak się ją czyta.

## Dobrze wiedzieć

- Więcej momentu niż seryjnie przy niskich obrotach podnosi ryzyko przedwczesnego zapłonu, który może za jednym razem zniszczyć tłoki.
- Zapłon wcześniejszy niż granica spalania stukowego nic nie daje: regulacja spalania stukowego go cofa.
- Lepsze paliwo samo daje zapas, nie moc: seryjna mapa zapłonu jest sufitem.
