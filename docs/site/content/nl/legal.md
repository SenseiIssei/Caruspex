# Veiligheid, merken en licenties

## Veiligheid en aansprakelijkheid

Caruspex is een vrij hobbyproject en komt zonder enige garantie, zoals de [MIT-licentie](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) zegt. Gebruik is op eigen risico.

- Bedien het nooit tijdens het rijden. Stel het in terwijl de auto stilstaat; een passagier kan de live-waarden in de gaten houden.
- Fouten wissen, actuatortests en basisinstellingen laten de auto iets doen. Caruspex vraagt vóór elk daarvan om bevestiging; doe ze alleen met de auto stilstaand en beveiligd, en alleen bij voertuigen waaraan je mag werken.
- Caruspex vervangt geen garage. Een foutcode noemt een symptoom, niet altijd de oorzaak. Remmen, airbags, stuurinrichting en alles wat voor de veiligheid telt, horen in deskundige handen.
- Wissen is geen reparatie. Gewiste fouten en teruggezette gereedheidstests kunnen een emissietest of APK laten mislukken.
- Caruspex verandert de kilometerstand, de startonderbreker, sleutels, emissiebeheer of een tachograaf niet, en het project neemt geen bijdragen aan die dat doen. Knoeien met kilometertellers, emissiebeheer of tachografen is in veel landen verboden.

## Merken

Namen van autofabrikanten, modellen, adapters, programma's en normen (bijvoorbeeld Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS of ISTA) zijn merken van hun respectieve eigenaars. Ze staan er alleen om te zeggen met welke auto's, adapters en interfaces Caruspex werkt. Caruspex is een onafhankelijk project en is niet verbonden aan, goedgekeurd door of gesponsord door een autofabrikant, adapterfabrikant of softwareleverancier. De app en deze repository gebruiken geen logo's van deze bedrijven.

## Licentie en bronnen

De code valt onder de MIT-licentie, zie [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). De bibliotheken in de programma's en hun licenties staan in [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); de app toont ze bij de instellingen. Gegevens die in de programma's zijn ingebouwd:

- Fabrikantwaarden voor ongeveer 550 voertuigen, omgezet uit [OBDb](https://github.com/OBDb), onder CC BY-SA 4.0 zoals OBDb: los van de code in [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) met de licentie en de lijst van bronnen ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- CAN-signaaldefinities uit [opendbc](https://github.com/commaai/opendbc) (comma.ai) en [model3dbc](https://github.com/joshwardell/model3dbc), MIT-licentie ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Motorcodes uit Wikipedia-artikelen, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), met artikel en versie in elke regel).
- Een deel van de fabrikantcodes van het VIN volgens de [WMI-lijst op Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), CC BY-SA 4.0.
- Afmetingen van voertuigen uit [Wikidata](https://www.wikidata.org), CC0.
- Lettertypen onder de SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
