# Kvaser (Windows)

Les adaptateurs CAN de Kvaser (Leaf, U100, Memorator et autres) s'utilisent directement sous Windows via la bibliothèque CANlib de Kvaser. Sous Linux, ce sont des interfaces [SocketCAN](socketcan.md).

## Connexion

1. Installez les pilotes Kvaser. Ils apportent `canlib32.dll`, que Caruspex charge.
2. Sur la page **Connecter**, choisissez **Interface CAN**. Chaque canal Kvaser avec un appareil derrière apparaît dans la liste ; les canaux virtuels du pilote sont écartés.
3. Choisissez le débit (500 kbit/s pour l'OBD sur CAN) et connectez.

## Ce qui fonctionne

L'OBD, tous les calculateurs via UDS, la recherche en parallèle des calculateurs, le VW TP 2.0 et l'écoute des valeurs diffusées. Le CAN FD via CANlib n'est pas encore intégré.

## Ligne de commande

```
caruspex_cli read --can kvaser:0@500000
```
