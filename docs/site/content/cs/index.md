# Caruspex

Caruspex je bezplatný diagnostický program pro auta a motorky. Běží na Windows, Linuxu a macOS, mluví 18 jazyky a funguje s adaptéry, které lidé opravdu mají: s levným ELM327, s OBDLinkem, s kabelem KKL pro starší auta koncernu Volkswagen, s kabelem ENET pro BMW a s profesionálními rozhraními CAN.

![Přehled s živými budíky](screenshot-overview.png)

## Co umí

- Čte a maže chybové kódy, vysvětluje je srozumitelně a řekne, jestli můžete jet dál.
- Ukazuje živé hodnoty: teploty, rychlost, směs, plnicí tlak, napětí baterie a mnohem víc, jako čísla i jako grafy.
- Ptá se každé řídicí jednotky v autě, nejen motoru, pokud to auto a adaptér dovolí.
- Prověří ojeté auto před koupí: sedí VIN, shodují se stavy tachometru, nemazaly se krátce předtím závady?
- Ukazuje auto jako skleněný model, ve kterém se závady a hodnoty rozsvítí tam, kam patří.
- Řekne vám, co dál: hned zastavit, brzy do servisu, nebo to počká.

## Kde začít

1. [Nainstalujte Caruspex](install.md) do počítače.
2. Projděte [první kroky](first-steps.md): vyzkoušejte demo auto a pak připojte to své.
3. Najděte svůj adaptér v části [adaptéry a rozhraní](interfaces.md).
4. V části [soukromí](privacy.md) si přečtěte, co Caruspex ukládá a posílá. Ve zkratce: nic neopustí váš počítač, pokud to sami nesdílíte.

> Caruspex jen čte. Nikdy nekóduje řídicí jednotky, nikdy nemění stav tachometru a nikdy nesahá na imobilizér. Mazání chybových kódů je jediná věc, kterou do auta zapisuje.

## Upřímný stav

Caruspex je postavený podle specifikací a testovaný proti simulovaným řídicím jednotkám, falešným gatewayům a falešným ovladačům. Na skutečném hardwaru byl zatím vyzkoušen jen OBDLink s čipem STN2232, a to bez auta. Pokud Caruspex vyzkoušíte na svém autě, pošlete prosím [zprávu o testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): každá zpráva prodlouží seznam otestovaných aut.
