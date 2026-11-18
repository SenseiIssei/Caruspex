# Adaptateurs et interfaces

Caruspex détermine tout seul ce qui est branché et ce que parle la voiture. Quel que soit le chemin vers la voiture, les mêmes fonctions marchent : OBD-II, tous les calculateurs via UDS, écoute des valeurs diffusées, enregistrement et relecture.

| Interface | Exemples | Page |
|---|---|---|
| ELM327 en USB | câbles ELM327 bon marché, Vgate, Veepeak | [ELM327 en USB](elm327-usb.md) |
| ELM327 en Bluetooth | boîtiers Bluetooth classique et Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 en Bluetooth](elm327-bluetooth.md) |
| ELM327 en Wi-Fi | boîtiers qui ouvrent leur propre Wi-Fi | [ELM327 en Wi-Fi](elm327-wifi.md) |
| Puces STN | OBDLink SX, EX, MX+, CX | [OBDLink et STN](obdlink-stn.md) |
| Câble KKL (VAG 409.1) | câbles FTDI ou CP2102 | [Câble KKL](kkl.md) |
| Câble K+DCAN | câbles BMW avec interrupteur | [Câble K+DCAN](kdcan.md) |
| Câble ENET | BMW et Mini à partir de la série F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable avec candleLight, cartes CAN pour Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Câble DoIP | beaucoup de voitures à partir d'environ 2016 | [DoIP](doip.md) |

## Que peut faire chacun ?

Un simple ELM327 suffit pour le moteur, la boîte de vitesses et, sur la plupart des voitures à partir d'environ 2008, tous les autres calculateurs via UDS. Il ne sait pas gérer l'ancien protocole du groupe Volkswagen KWP1281, qui acquitte chaque octet : pour VW, Audi, Seat et Skoda d'avant 2004 environ, il vous faut un câble KKL.

Les interfaces CAN brutes (J2534, SLCAN, SocketCAN, PCAN, Kvaser) donnent à Caruspex les trames CAN nues. Il gère alors lui-même le protocole de transport, ce qui permet une recherche rapide en parallèle des calculateurs et le VW TP 2.0 pour les voitures du groupe Volkswagen d'environ 2004 à 2012.

![Test de l'adaptateur](screenshot-adapter.png)

## Mon adaptateur est-il authentique ?

Le **Test de l'adaptateur** envoie à l'adaptateur les commandes de chaque version d'ELM327, mesure les temps de réponse et dit honnêtement ce qu'on peut en conclure : beaucoup d'adaptateurs bon marché annoncent la version 1.5 ou 2.1 mais ne connaissent que les commandes de la 1.0. Caruspex n'écrit jamais rien dans l'adaptateur.

> Testé jusqu'ici : un OBDLink avec la puce STN2232 a passé le test de l'adaptateur, sans voiture. Tous les autres adaptateurs et interfaces sont développés d'après leur documentation mais pas encore testés sur une vraie voiture.
