# SLCAN (Lawicel)

SLCAN est un protocole texte simple sur port série pour les clés CAN : CANable avec le firmware slcan, USBtin, Lawicel CANUSB et beaucoup d'appareils faits maison. Il fonctionne sous Windows, Linux et macOS.

## Connexion

1. Branchez la clé sur l'ordinateur et, avec un câble, sur la prise OBD : CAN high sur la broche 6, CAN low sur la broche 14, masse sur les broches 4 et 5.
2. Sur la page **Connecter**, choisissez **Interface CAN**. Les clés SLCAN sont trouvées par la recherche automatique des ports et apparaissent dans la liste.
3. Choisissez le débit (500 kbit/s pour l'OBD sur CAN) et connectez.

## Ce qui fonctionne

Comme avec toute interface CAN brute : l'OBD, tous les calculateurs via UDS, la recherche en parallèle des calculateurs, le VW TP 2.0 et l'écoute des valeurs diffusées. Le CAN FD n'est pas possible via SLCAN.

## Conseils

- Un CANable avec le firmware candleLight n'est pas un appareil SLCAN : sous Linux, il apparaît comme une interface [SocketCAN](socketcan.md).
- La clé n'a pas besoin de résistance de terminaison sur la prise OBD, la voiture a la sienne.

## Ligne de commande

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
