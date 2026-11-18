# Kabel K+DCAN

Kabel K+DCAN je obvyklý kabel pro BMW zhruba z let 1998 až 2017 s dílenskými programy INPA a ISTA. Uvnitř je sériový čip FTDI a malý převodník pro D-CAN od BMW. Mnoho těchto kabelů má pro starší auta přepínač nebo propojku mezi piny 7 a 8.

## Stav v Caruspexu

> Upřímný stav: Caruspex zatím neumí režim D-CAN tohoto kabelu. Staré protokoly BMW (DS2 a KWP2000 ve vlastním pojetí BMW) přes K-Line jsou v plánu, ale nejdřív je potřeba je otestovat na skutečném autě.

Co můžete místo toho použít už dnes:

- **BMW a Mini od řady F (zhruba od roku 2008)**: [kabel ENET](enet.md) se dostane ke každé řídicí jednotce přes Ethernet, nebo obyčejný [ELM327](elm327-usb.md) s vlastním adresováním BMW přes CAN.
- **Standardní OBD u BMW s CAN (zhruba od roku 2007)**: každý ELM327 přečte závady motoru a živé hodnoty.
- **V režimu K-Line** propouští kabel K-Line jako [kabel KKL](kkl.md). To se hodí pro auta koncernu VW; řídicí jednotky BMW na K-Line potřebují protokoly, které zatím chybí.

## Hledáme pomoc

Pokud máte kabel K+DCAN a starší BMW, [zpráva o testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) se záznamem pomůže tuto část postavit.
