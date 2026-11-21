# Živé hodnoty

Stránka **Živé hodnoty** (profesionální režim) ukazuje, co snímače právě měří. V režimu pro začátečníky ukazuje **Režim jízdy** nejdůležitější hodnoty velkým písmem.

![Živé hodnoty](screenshot-live.png)

## Co tu je

- Každá hodnota módu 01 podle SAE J1979 až po PID C4: 145 PID s více než 300 údaji, seskupené a s vyhledáváním. Které z nich vaše auto nabízí, řekne samo.
- Stav tachometru, rychlostní stupeň, teplota oleje, spotřeba paliva, točivý moment, plnicí tlak, teploty výfukových plynů, filtr pevných částic, AdBlue, NOx a hybridní baterie, pokud je auto hlásí.
- Průběžný graf pro několik hodnot najednou. U dlouhých oken (až hodina) graf staré body prořídí, ale zachová každou špičku i každý propad.
- Nahrávání do CSV.

## Hodnoty výrobce a vlastní hodnoty

- **Baterie elektromobilů**: stav baterie, nabití, napětí článků s rozptylem a teploty pro Hyundai Kona a Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf a Tesla Model 3 a Y (odposlechem na sběrnici CAN), na stránce **Baterie elektromobilu**.
- **Hodnoty OBDb** pro zhruba 550 dalších vozidel (tlak v pneumatikách, olej, stav tachometru a další), nabízené samy pro rozpoznané auto.
- **Rozšířená diagnostická relace**: některé hodnoty dieselů koncernu Volkswagen, například naměřenou hmotnost sazí a vzdálenost od poslední regenerace, vydává řídicí jednotka motoru jen ve své rozšířené diagnostické relaci. V režimu pro profesionály ji do ní Caruspex přepne, dokud se takové hodnoty dotazují, jen kvůli čtení, označí je **Relace 03** a pak ji přepne zpět. Vypnout se to dá v nastavení v části **Hodnoty výrobce**. Když jednotka odmítne, například za jízdy, u hodnoty je uveden důvod.
- **Vaše vlastní hodnoty** v rozšířeném formátu CSV z Torque, včetně rovnic.

![Baterie elektromobilu](screenshot-battery.png)

## Jak rychle?

Kolik hodnot za sekundu dorazí, záleží na autě a adaptéru. Auta s CAN odpovídají rychle; auta na K-Line jsou z principu pomalá. S OBDLinkem se Caruspex může ptát až na 24 hodnot najednou (viz [OBDLink a STN](obdlink-stn.md)); přímé rozhraní CAN je nejrychlejší cesta.

## Nahrávání a přehrávání

Celé sezení s autem lze nahrát do souboru a později přehrát bez auta, což se hodí při hledání závad. Tlačítkem **Sdílet záznam** lze připravit záznam pro projekt bez osobních údajů, viz [soukromí](privacy.md).
