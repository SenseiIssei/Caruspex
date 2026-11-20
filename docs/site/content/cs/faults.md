# Závady

Stránka **Paměť závad** přečte, co řídicí jednotky uložily, a srozumitelně to vysvětlí.

![Paměť závad](screenshot-faults.png)

## Co uvidíte

- **Uložené, čekající a trvalé** kódy. Při více řídicích jednotkách u každého kódu vidíte, kdo ho nahlásil (motor, převodovka).
- Zhruba 3 400 standardních kódů (P, C, B, U) a kódy výrobců VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai a Kia, Subaru a dalších. Kód P1 se čte tak, jak ho myslí rozpoznaná značka.
- **Semafor** u každého kódu a otázka, kterou si každý položí jako první: můžu jet dál?
- Obvyklé příčiny, věci, které si můžete zkontrolovat sami, a hrubá cenová třída.
- Kde díl sedí: auto v průhledném zobrazení, nebo [rentgenový pohled](xray.md).

## Všechny řídicí jednotky

V profesionálním režimu se stránka **Všechny řídicí jednotky** zeptá každé řídicí jednotky, nejen motoru: ABS, airbagu, přístrojového panelu, klimatizace a dalších. Každá závada přichází s typem poruchy (zkrat, přerušení, nevěrohodný signál), číslem dílu a verzí softwaru. Starší auta koncernu VW se čtou přes [kabel KKL](kkl.md).

## Co mám teď dělat?

Stránka **Co dělat?** roztřídí všechno, co Caruspex našel, do tří kroků: hned zastavit, brzy do servisu, počká to. Každý bod má jednu jednoduchou větu proč, hrubé cenové rozpětí a poznámku, kterou můžete ukázat v servisu. OBD nevidí, jestli kontrolka motoru bliká nebo svítí červená výstražná kontrolka, proto se vás na to ptají dvě políčka; blikající kontrolka s vynecháváním zapalování přesune bod do "hned zastavit".

![Co mám teď dělat](screenshot-todo.png)

## Mazání kódů

Mazat lze u každé řídicí jednotky zvlášť, s jasným varováním, a Caruspex hned poté paměť přečte znovu.

> Smazání není oprava. Vynuluje také testy připravenosti: dokud auto neujede několik úplných jízdních cyklů, neprojde měřením emisí, a kód, jehož příčina pořád trvá, se po krátké jízdě vrátí.

## Export

Seznam lze uložit jako CSV a zprávu z kontroly jako PDF.
