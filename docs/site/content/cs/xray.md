# Rentgenový pohled

**Rentgenový pohled** ukazuje připojené auto jako skleněný model, kterým můžete otáčet. Uvnitř vidíte motor nebo elektromotory, převodovku, nádrž nebo vysokonapěťovou baterii, výfuk, chladič, 12V baterii a zásuvku OBD, rozmístěné podle typu karoserie a pohonu rozpoznaného auta.

![Rentgenový pohled](screenshot-xray.png)

## Co se rozsvítí

- **Závady** obarví svou oblast a odpovědná řídicí jednotka zčervená.
- **Živé hodnoty** visí jako štítky na dílu, ke kterému patří: chladicí kapalina u chladiče, otáčky u motoru, napětí u baterie.
- **Řídicí jednotky** nalezené hledáním sedí tam, kde obvykle bývají, propojené svými datovými vodiči (CAN pohonu, komfortní CAN, diagnostický CAN, K-Line). Zatímco Caruspex čte, putují k dotazované jednotce malé balíčky.
- **Zkontrolovat auto** proletí kamerou ke každému nálezu, jednomu po druhém.

Otáčejte myší nebo prstem, přibližujte kolečkem nebo dvěma prsty; funguje i klávesnice. Vrstvy (karoserie, pohon, chlazení, palivo nebo vysoké napětí, elektrika, síť) lze vypínat jednotlivě.

## Jak přesný je model?

Tvar vychází ze skutečných rozměrů generace, pokud jsou známé (z Wikidata nebo z technických údajů výrobce), jinak z typických hodnot pro typ karoserie a dobu. Není to věrná kopie přesného modelu: konstrukční data a schémata zapojení výrobců nejsou volně dostupná. Stránka uvádí, odkud tvar pochází.

Pokud máte 3D model svého auta ve formátu glTF (`.glb` nebo `.gltf`), můžete ho načíst tlačítkem **Načíst vlastní 3D model**; vnitřek zůstane podle rozvržení.

## Bez 3D

Na počítačích bez WebGL přejde stránka na ploché boční zobrazení se stejnými informacemi.
