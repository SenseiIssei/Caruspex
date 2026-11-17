# OBDLink et puces STN

Les adaptateurs OBDLink (SX, EX, MX+, CX, LX) utilisent des puces STN d'OBD Solutions. Ils comprennent tout ce que comprend un ELM327, et plus encore ; Caruspex les reconnaît tout seul et utilise leurs fonctions supplémentaires.

## Ce que Caruspex utilise

- **Requêtes groupées** : Caruspex demande jusqu'à 24 valeurs en direct d'un coup au lieu de 6. En Bluetooth, un tour de valeurs devient ainsi presque deux fois plus rapide.
- **Écoute** : les valeurs diffusées (par exemple sur une Tesla) sont enregistrées avec les commandes propres au STN, qui s'arrêtent d'elles-mêmes après un nombre donné de messages.
- **Ford MS-CAN et GM Single-Wire-CAN** : les calculateurs de carrosserie de Ford et GM ne sont pas sur le CAN de diagnostic normal. Avec un adaptateur qui a l'émetteur-récepteur supplémentaire, la page **Tous les calculateurs** propose "Ford : calculateurs de carrosserie sur MS-CAN" ou "GM : calculateurs de carrosserie sur Single-Wire-CAN".

| Puce | Adaptateur | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 à STN2232 | OBDLink EX | oui | non |
| STN1150 à 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ et autres | oui | oui |
| autres | OBDLink SX, LX, CX | non | non |

## Connexion

Les adaptateurs OBDLink se connectent comme n'importe quel ELM327 : en USB (l'EX et le SX), en Bluetooth (MX+, LX, CX) ou comme décrit dans [ELM327 en USB](elm327-usb.md) et [ELM327 en Bluetooth](elm327-bluetooth.md).

## Testé

Un OBDLink avec la puce STN2232 a passé le test de l'adaptateur : puce, firmware et commandes ont été correctement reconnus. Il n'a pas encore été utilisé sur une voiture, donc les requêtes groupées, le MS-CAN et le SW-CAN ne sont pas encore confirmés sur du vrai matériel.
