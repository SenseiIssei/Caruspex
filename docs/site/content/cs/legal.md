# Bezpečnost, ochranné známky a licence

## Bezpečnost a odpovědnost

Caruspex je svobodný hobby projekt bez jakékoli záruky, jak říká [licence MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Používáte ho na vlastní nebezpečí.

- Nikdy ho neovládejte za jízdy. Nastavte ho ve stojícím voze; živé hodnoty může sledovat spolujezdec.
- Mazání závad, testy akčních členů a základní nastavení nutí auto něco dělat. Caruspex se před každým z nich zeptá; provádějte je jen ve stojícím a zajištěném voze a jen u vozidel, na kterých smíte pracovat.
- Caruspex nenahrazuje servis. Kód závady ukazuje příznak, ne vždy příčinu. Brzdy, airbagy, řízení a vše, na čem závisí bezpečnost, patří do odborných rukou.
- Smazání není oprava. Smazané závady a vynulované testy připravenosti mohou způsobit neúspěch při měření emisí nebo technické kontrole.
- Caruspex nemění stav tachometru, imobilizér, klíče, systém čištění výfukových plynů ani tachograf a projekt nepřijímá příspěvky, které by to dělaly. Manipulace s počítadly kilometrů, čištěním výfukových plynů nebo tachografy je v mnoha zemích nezákonná.

## Ochranné známky

Názvy výrobců vozidel, modelů, adaptérů, programů a norem (například Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS nebo ISTA) jsou ochranné známky jejich vlastníků. Uvádějí se jen proto, aby bylo jasné, s jakými auty, adaptéry a rozhraními Caruspex pracuje. Caruspex je nezávislý projekt a není spojen s žádným výrobcem vozidel, výrobcem adaptérů ani dodavatelem softwaru, není jimi schválen ani sponzorován. Aplikace a tento repozitář nepoužívají loga těchto firem.

## Licence a zdroje

Kód je pod licencí MIT, viz [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Knihovny obsažené v programech a jejich licence jsou uvedeny v [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); aplikace je ukazuje v nastavení. Data zabudovaná do programů:

- Hodnoty výrobců pro asi 550 vozidel, převedené z [OBDb](https://github.com/OBDb), stejně jako OBDb pod CC BY-SA 4.0: odděleně od kódu v [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) s licencí a seznamem zdrojů ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Definice signálů CAN z [opendbc](https://github.com/commaai/opendbc) (comma.ai) a [model3dbc](https://github.com/joshwardell/model3dbc), licence MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Kódy motorů z článků Wikipedie, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), s článkem a verzí v každém řádku).
- Část kódů výrobců z VIN podle [seznamu WMI na Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), CC BY-SA 4.0.
- Rozměry vozidel z [Wikidat](https://www.wikidata.org), CC0.
- Písma pod licencí SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
