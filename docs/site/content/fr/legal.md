# Sécurité, marques et licences

## Sécurité et responsabilité

Caruspex est un projet de loisir libre, fourni sans aucune garantie, comme le dit la [licence MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Vous l'utilisez à vos propres risques.

- Ne l'utilisez jamais en conduisant. Préparez-le véhicule à l'arrêt ; un passager peut surveiller les valeurs en direct.
- L'effacement des défauts, les tests d'actionneurs et les réglages de base font agir la voiture. Caruspex demande avant chacun d'eux ; ne les lancez que véhicule à l'arrêt et immobilisé, et uniquement sur des véhicules sur lesquels vous avez le droit d'intervenir.
- Caruspex ne remplace pas un garage. Un code défaut indique un symptôme, pas toujours sa cause. Freins, airbags, direction et tout ce qui compte pour la sécurité doivent être confiés à des professionnels.
- Effacer n'est pas réparer. Des défauts effacés et des moniteurs de disponibilité remis à zéro peuvent faire échouer un contrôle antipollution ou technique.
- Caruspex ne modifie ni le kilométrage, ni l'antidémarrage, ni les clés, ni la dépollution, ni le chronotachygraphe, et le projet n'accepte aucune contribution qui le ferait. Manipuler un compteur kilométrique, la dépollution ou un chronotachygraphe est illégal dans de nombreux pays.

## Marques

Les noms de constructeurs, de modèles, d'adaptateurs, de logiciels et de normes (par exemple Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS ou ISTA) sont des marques de leurs propriétaires respectifs. Ils n'apparaissent que pour dire avec quels véhicules, adaptateurs et interfaces Caruspex fonctionne. Caruspex est un projet indépendant, qui n'est ni affilié, ni approuvé, ni soutenu par un constructeur, un fabricant d'adaptateurs ou un éditeur de logiciels. L'application et ce dépôt n'utilisent aucun logo de ces entreprises.

## Licence et sources

Le code est sous licence MIT, voir [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Les bibliothèques contenues dans les programmes et leurs licences figurent dans [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md) ; l'application les affiche dans les réglages. Données intégrées aux programmes :

- Valeurs constructeur pour environ 550 véhicules, converties à partir d'[OBDb](https://github.com/OBDb), sous CC BY-SA 4.0 comme OBDb : séparées du code dans [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) avec la licence et la liste des sources ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Définitions de signaux CAN issues d'[opendbc](https://github.com/commaai/opendbc) (comma.ai) et de [model3dbc](https://github.com/joshwardell/model3dbc), licence MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Codes moteur issus d'articles de Wikipédia, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), avec l'article et la révision sur chaque ligne).
- Une partie des codes constructeur du VIN d'après la [liste WMI de Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), CC BY-SA 4.0.
- Dimensions des véhicules issues de [Wikidata](https://www.wikidata.org), CC0.
- Polices sous SIL Open Font License 1.1 : JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
