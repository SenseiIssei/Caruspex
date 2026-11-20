# Soukromí

Caruspex pracuje ve vašem počítači. Nemá účet, cloud ani sledování.

## Co se ukládá

Všechno zůstává ve vašem počítači:

| Co | Kdy | Kde |
|---|---|---|
| Nastavení | vždy | složka nastavení vašeho systému (ukáže ji tlačítko **Otevřít umístění úložiště** v nastavení) |
| Garáž: VIN, auto, kontroly, stavy tachometru, poslední fungující připojení | po připojení auta | vedle nastavení, ve složce dat Caruspexu |
| Záznamy a soubory CSV | jen když je spustíte | tam, kam je uložíte |
| Soubor protokolu | jen pokud ho zapnete v nastavení | cesta, kterou si zvolíte |
| Zprávy o pádu | jen když Caruspex spadne | složka `crash-reports` vedle garáže |
| Zprávy z prvního testu KKL | jen když nějakou uložíte | složka `test-reports` ve složce nastavení; VIN a číslo imobilizéru se zkracují, pokud nezaškrtnete políčko |

Kterýkoli z těchto souborů můžete kdykoli smazat.

## Co se odesílá

Při každé instalaci, i po aktualizaci ze starší verze, se Caruspex jednou zeptá, zda má hledat aktualizace. Teprve po odpovědi "ano" se zeptá GitHubu, zda existuje novější verze, při spuštění a pak každých šest hodin, dokud běží. Nic o vás ani o autě se přitom neodesílá. Bez "ano" nejde na GitHub žádný požadavek. Lze to kdykoli změnit v nastavení v části **Okno a jazyk**. Jinak se Caruspex k internetu nepřipojuje. Komunikace s adaptérem Wi-Fi, kabelem ENET nebo autem s DoIP zůstává ve vaší místní síti.

Aktualizace hledají jen verze zveřejněné na GitHubu; kopie, kterou si sestavíte sami, to nedělá nikdy. Když Caruspex hledá aktualizaci nebo ji stahuje, vidí GitHub jako u každého stahování IP adresu počítače a verzi Caruspexu; Caruspex sám neodesílá ani neukládá nic dalšího a nikdo jiný, ani projekt, to nevidí. Platí pro to prohlášení o ochraně soukromí GitHubu. Aktualizace se stahují jen z vydání repozitáře Caruspex. Tlačítka jako **Nahlásit na GitHubu** jen otevřou prohlížeč; co tam odešlete, je na vás.

## Android

Do Androidu 11 systém dovolí aplikaci najít adaptéry Bluetooth LE jen s oprávněním k poloze. Caruspex o něj žádá jen z tohoto důvodu a vaši polohu nikdy nečte. Od Androidu 12 žádá místo toho o oprávnění Bluetooth, označené jako nikdy nepoužívané pro polohu.

## Zprávy o pádu jen na vyžádání

Když Caruspex spadne, zapíše do vašeho počítače krátkou zprávu: verzi, systém, místo v kódu a řetězec volání. Čísla VIN, bajty komunikace s autem a vaše domovská složka se vystřihnou. Nic se neodesílá. Caruspex vám zprávu ukáže a nabídne dvě tlačítka: **Nahlásit na GitHubu** otevře v prohlížeči předvyplněné hlášení, které si přečtete a upravíte, než ho zveřejníte, **Nehlásit** ho odloží.

## Sdílení záznamů bez osobních údajů

Záznamy ze skutečných aut pomáhají Caruspex vylepšit pro víc aut. **Sdílet záznam** připraví kopii a přesně ukáže, co zůstane a co zmizí:

- **Odstraněno**: poslední číslice VIN (všude v souboru), název portu a adresa Bluetooth nebo síťová adresa adaptéru, denní čas.
- **Nikdy v záznamu**: vaše poloha a uživatelská data.
- **Zůstává**: dotazy a odpovědi, typ adaptéru, protokol a datum.

Soubor zůstane ve vašem počítači, dokud ho sami neuložíte a nepřiložíte k hlášení na GitHubu. Nic se nenahrává automaticky.

## Testovací program

[Formulář zprávy o testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) se ptá na auto, rok výroby, motor a adaptér. Chce jen zkrácený VIN; nikdy nezveřejňujte celý VIN, registrační značku ani své jméno.
