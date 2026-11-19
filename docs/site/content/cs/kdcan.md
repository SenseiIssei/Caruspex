# Kabel K+DCAN

Kabel K+DCAN je obvyklý kabel pro BMW zhruba z let 1998 až 2017 s dílenskými programy INPA a ISTA. Uvnitř je sériový čip FTDI a malý převodník pro D-CAN od BMW. Mnoho těchto kabelů má pro starší auta přepínač nebo propojku mezi piny 7 a 8.

## Stav v Caruspexu

> Upřímný stav: Caruspex čte BMW řady E přes K-Line tohoto kabelu: DS2 u starších modelů, KWP2000 ve stylu BMW a BMW-FAST u generace E60. Zatím je to vyzkoušené jen na simulovaných autech v programu, ještě ne na skutečném BMW. Režim D-CAN kabelu Caruspex zatím neumí.

## V režimu K-Line

- **Přepínač na K-Line** dejte dřív, než kabel zasunete do zásuvky; u většiny kabelů je to levá poloha. Propojí piny 7 a 8, aby odpovídaly i řídicí jednotky na pinu 8 u starších modelů.
- **Která auta**: E36 (s kulatým 20pinovým konektorem přes redukci), E38, E39, E46, E52, E53, E83, E85 a generace E60 (E60, E61, E63, E64, E65, E87, E90) do 03/2007. Potom a od řady F mluví zásuvka D-CAN nebo Ethernetem.
- **Jak na to**: zapnout profesionální režim, stránka *BMW přes K-Line*, vybrat port, *Prohledat*. Caruspex se ptá každé známé adresy přes DS2 (9600 Bd, sudá parita), KWP2000 (10400 Bd s rychlou inicializací) a BMW-FAST (115200 Bd) a ukáže nalezené řídicí jednotky s identifikací a pamětí závad. Trvá to asi půl minuty.
- **Mazání** jde u jedné jednotky nebo všude, vždy až po dotazu. Potom Caruspex přečte, co zůstalo.
- **Kódy závad**: BMW ke svým kódům nezveřejňuje texty, proto je Caruspex ukazuje jako šestnáctkové číslo.
- **Jen čtení**: kódování, přizpůsobení ani testy akčních členů tu nejsou; ty patří do dílenského softwaru.
- **Kabel KKL** funguje stejně, ale u starších modelů dosáhne jen na jednotky na pinu 7, dokud piny 7 a 8 nejsou propojené.

Na vyzkoušení bez auta je v seznamu portů na stránce simulované E46 (DS2) a E60 (KWP2000).

## Co ještě funguje

- **BMW a Mini od řady F (zhruba od roku 2008)**: [kabel ENET](enet.md) se dostane ke každé řídicí jednotce přes Ethernet, nebo obyčejný [ELM327](elm327-usb.md) s vlastním adresováním BMW přes CAN.
- **Standardní OBD u BMW s CAN (zhruba od roku 2007)**: každý ELM327 přečte závady motoru a živé hodnoty.

## Hledáme pomoc

Pokud máte kabel K+DCAN a starší BMW, [zpráva o testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) s protokolem pomůže potvrdit část K-Line na skutečném autě.
