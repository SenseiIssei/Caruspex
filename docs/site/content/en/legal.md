# Safety, trademarks and licences

## Safety and liability

Caruspex is a free hobby project and comes without any warranty, as the [MIT licence](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) says. You use it at your own risk.

- Never operate it while driving. Set it up with the car parked; a passenger can watch the live values.
- Clearing faults, output tests and basic settings make the car do something. Caruspex asks before each of them; do them only with the car parked and secured, and only on vehicles you are allowed to work on.
- Caruspex is not a substitute for a workshop. A fault code names a symptom, not always its cause. Brakes, airbags, steering and everything that matters for safety belong in trained hands.
- Clearing is not a repair. Cleared faults and reset readiness monitors can make an emissions or roadworthiness test fail.
- Caruspex does not change the mileage, the immobiliser, keys, emissions control or a tachograph, and the project does not accept contributions that do. Tampering with odometers, emissions control or tachographs is illegal in many countries.

## Trademarks

Names of vehicle makers, models, adapters, programs and standards (for example Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS or ISTA) are trademarks of their respective owners. They appear only to say which cars, adapters and interfaces Caruspex works with. Caruspex is an independent project and is not affiliated with, endorsed by or sponsored by any vehicle maker, adapter maker or software vendor. The app and this repository use no logos of these companies.

## License and credits

The code is under the MIT licence, see [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). The libraries in the programs and their licences are listed in [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); the app shows them under Settings. Data built into the programs:

- Manufacturer values for about 550 vehicles, converted from [OBDb](https://github.com/OBDb), under CC BY-SA 4.0 like OBDb: kept apart from the code in [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) with the licence and the list of sources ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- CAN signal definitions from [opendbc](https://github.com/commaai/opendbc) (comma.ai) and [model3dbc](https://github.com/joshwardell/model3dbc), MIT licence ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Battery values for the Hyundai Ioniq and Kona, the Kia e-Niro, the Nissan Leaf and the Renault Zoe after OBDb (CC BY-SA 4.0, like these packs), the [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3) and [evDash](https://github.com/nickn17/evDash) (both MIT): [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- Engine codes from Wikipedia articles, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), with article and revision in every row).
- VIN manufacturer codes checked against [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (public domain) and [Wikidata](https://www.wikidata.org) (CC0); the codes that only the [WMI list on Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) has are under CC BY-SA 4.0 and kept apart in [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi).
- Vehicle dimensions from [Wikidata](https://www.wikidata.org), CC0.
- Fonts under the SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
