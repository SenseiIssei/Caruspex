# Câble K+DCAN

Le câble K+DCAN est le câble habituel pour les BMW d'environ 1998 à 2017 avec les logiciels d'atelier INPA et ISTA. À l'intérieur se trouvent une puce série FTDI et un petit convertisseur pour le D-CAN de BMW. Beaucoup de ces câbles ont un interrupteur ou un pont entre les broches 7 et 8 pour les voitures plus anciennes.

## État dans Caruspex

> État honnête : Caruspex ne parle pas encore le mode D-CAN de ce câble. Les anciens protocoles BMW (DS2 et KWP2000 à la façon de BMW) sur la K-Line sont prévus mais doivent d'abord être testés sur une vraie voiture.

Ce que vous pouvez utiliser à la place dès aujourd'hui :

- **BMW et Mini à partir de la série F (environ 2008 et après)** : un [câble ENET](enet.md) atteint tous les calculateurs par Ethernet, ou un simple [ELM327](elm327-usb.md) avec l'adressage BMW sur CAN.
- **OBD standard sur les BMW avec CAN (environ 2007 et après)** : n'importe quel ELM327 lit les défauts moteur et les valeurs en direct.
- **En mode K-Line**, le câble laisse passer la K-Line comme un [câble KKL](kkl.md). C'est utile pour les voitures du groupe VW ; les calculateurs BMW sur K-Line ont besoin des protocoles qui manquent encore.

## Appel à l'aide

Si vous avez un câble K+DCAN et une BMW plus ancienne, un [rapport de test](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) avec un enregistrement aide à construire cette partie.
