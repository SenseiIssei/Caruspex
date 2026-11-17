# Časté otázky

## Stojí Caruspex něco?

Ne. Caruspex je svobodný software pod licencí MIT, bez reklam, účtů a placených doplňků.

## Jaký adaptér si mám koupit?

Pro většinu lidí je nejlepší začátek ELM327 na USB s čipem FTDI nebo CH340, nebo OBDLink (Bluetooth nebo USB). Pro VW, Audi, Seat a Škodu zhruba před rokem 2004 přidejte [kabel KKL](kkl.md); pro BMW a Mini od řady F [kabel ENET](enet.md). [Přehled rozhraní](interfaces.md) je porovnává.

## Bude to fungovat s mým autem?

Každé auto s OBD-II nebo EOBD funguje pro motor: v Evropě benzínová auta od roku 2001 a diesely od roku 2004, v USA od roku 1996. Starší auta koncernu VW zhruba od roku 1990 fungují přes kabel KKL. Stránka **Vozidla** vám řekne, která cesta funguje u konkrétního auta. Motorky od Euro 5 fungují s kabelem adaptéru pro jejich diagnostickou zásuvku.

## Bylo to testované na mém autě?

Upřímně: zatím ne. Caruspex se testuje proti simulovaným řídicím jednotkám; na skutečném hardwaru byl zatím vyzkoušen jen OBDLink s čipem STN2232, a to bez auta. Pomozte prosím [zprávou o testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Může Caruspex moje auto poškodit?

Caruspex jen čte. Jediné, co zapisuje, je mazání chybových kódů, a než to udělá, varuje vás. Nikdy nekóduje, nikdy neflashuje a nikdy nemění adaptační hodnoty. Nepoužívejte ho za jízdy.

## Umí změnit stav tachometru, imobilizér nebo nakódovat funkce?

Ne, a nebude. Je to právně citlivé a není to účelem tohoto nástroje.

## Proč adaptér hlásí verzi 2.1, ale některé věci selhávají?

Mnoho levných adaptérů hlásí novější verzi ELM327, než ve skutečnosti mají. **Test adaptéru** zjistí, které příkazy adaptér opravdu zná.

## Proč jsou hodnoty u mého staršího auta pomalé?

Auta na K-Line (v Evropě zhruba 2001 až 2007) odpovídají z principu pomalu. Auta s CAN jsou mnohem rychlejší.

## Posílá Caruspex moje data někam?

Ne. Jen kontrola aktualizací se ptá GitHubu na nejnovější verzi a lze ji vypnout. Viz [soukromí](privacy.md).

## Jak můžu pomoct?

Pošlete zprávu o testu ze svého auta, sdílejte záznam, hlaste chyby nebo vylepšete překlad: návod je [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md) a tlačítko **Vylepšit překlad** v nastavení ho otevře.
