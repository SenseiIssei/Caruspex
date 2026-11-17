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

Kterýkoli z těchto souborů můžete kdykoli smazat.

## Co se odesílá

Při spuštění se Caruspex zeptá GitHubu, jestli existuje novější verze. Nic o vás ani o autě se přitom neodesílá. Kontrolu lze vypnout v nastavení. Jinak se Caruspex k internetu nepřipojuje. Komunikace s adaptérem Wi-Fi, kabelem ENET nebo autem s DoIP zůstává ve vaší místní síti.

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
