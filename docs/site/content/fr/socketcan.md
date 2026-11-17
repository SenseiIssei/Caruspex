# SocketCAN (Linux)

SocketCAN est la couche CAN du noyau Linux. Tout adaptateur CAN avec un pilote noyau peut s'utiliser par son intermédiaire : PEAK PCAN-USB, Kvaser, CANable avec le firmware candleLight, cartes CAN pour Raspberry Pi et bien d'autres.

## Mise en place

Activez l'interface avec le débit de la voiture (500 kbit/s pour l'OBD sur CAN) :

```
sudo ip link set can0 up type can bitrate 500000
```

Pour le CAN FD sur les voitures les plus récentes, avec en plus un débit de données :

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Connexion

Sur la page **Connecter**, choisissez **Interface CAN**. Chaque interface CAN de l'ordinateur figure dans la liste (`can0`, `can1` et ainsi de suite). Sélectionnez-la, choisissez le débit et, pour le CAN FD, cochez la case CAN FD.

## Ce qui fonctionne

L'OBD, tous les calculateurs via UDS, la recherche en parallèle des calculateurs, le VW TP 2.0, l'écoute des valeurs diffusées et le CAN FD.

## Ligne de commande

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
