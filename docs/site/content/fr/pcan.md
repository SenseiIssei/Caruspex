# PEAK PCAN (Windows)

Les adaptateurs CAN de PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro et autres) s'utilisent directement sous Windows, sans pilote J2534. Sous Linux, ce sont des interfaces [SocketCAN](socketcan.md).

## Connexion

1. Installez le pilote de périphérique de PEAK. Il apporte la bibliothèque `PCANBasic.dll`, que Caruspex charge.
2. Sur la page **Connecter**, choisissez **Interface CAN**. Chaque canal PCAN avec un appareil derrière apparaît tout seul dans la liste ; les canaux déjà utilisés par un autre programme sont écartés.
3. Choisissez le débit (500 kbit/s pour l'OBD sur CAN) et connectez.

## Ce qui fonctionne

L'OBD, tous les calculateurs via UDS, la recherche en parallèle des calculateurs, le VW TP 2.0 et l'écoute des valeurs diffusées. Le CAN FD via PCAN-Basic n'est pas encore intégré.

## Ligne de commande

```
caruspex_cli read --can pcan:USB1@500000
```
