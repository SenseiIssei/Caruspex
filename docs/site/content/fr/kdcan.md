# Câble K+DCAN

Le câble K+DCAN est le câble habituel pour les BMW d'environ 1998 à 2017 avec les logiciels d'atelier INPA et ISTA. À l'intérieur se trouvent une puce série FTDI et un petit convertisseur pour le D-CAN de BMW. Beaucoup de ces câbles ont un interrupteur ou un pont entre les broches 7 et 8 pour les voitures plus anciennes.

## État dans Caruspex

> État réel : Caruspex lit les BMW série E par la K-Line de ce câble : DS2 sur les modèles anciens, KWP2000 à la manière de BMW et BMW-FAST sur la génération E60. Pour l'instant, cela n'a été testé qu'avec les voitures simulées du programme, pas encore sur une vraie BMW. Caruspex ne parle pas encore le mode D-CAN du câble.

## En mode K-Line

- **Interrupteur sur K-Line** avant de brancher le câble dans la prise ; sur la plupart des câbles, c'est la position de gauche. Elle relie les broches 7 et 8, pour que les calculateurs sur la broche 8 des modèles anciens répondent aussi.
- **Quelles voitures** : E36 (avec la prise ronde à 20 broches via un adaptateur), E38, E39, E46, E52, E53, E83, E85 et la génération E60 (E60, E61, E63, E64, E65, E87, E90) jusqu'à 03/2007. Ensuite, et à partir de la série F, la prise parle D-CAN ou Ethernet.
- **Comment** : mode Pro activé, page *BMW via K-Line*, choisir le port, *Rechercher*. Caruspex interroge chaque adresse connue en DS2 (9600 bauds, parité paire), KWP2000 (10400 bauds avec initialisation rapide) et BMW-FAST (115200 bauds) et affiche les calculateurs trouvés avec leur identification et leur mémoire de défauts. Cela dure environ une demi-minute.
- **Effacer** se fait par calculateur ou partout, toujours après confirmation. Ensuite, Caruspex relit ce qui reste.
- **Codes défaut** : BMW ne publie pas de textes pour ses codes, Caruspex les affiche donc en hexadécimal.
- **Lecture seule** : pas de codage, d'adaptation ni de test d'actionneurs ici ; cela relève du logiciel d'atelier.
- **Un câble KKL** fonctionne de la même façon, mais sur les modèles anciens il n'atteint que les calculateurs sur la broche 7 tant que les broches 7 et 8 ne sont pas reliées.

Pour essayer sans voiture, la liste des ports de la page contient une E46 (DS2) et une E60 (KWP2000) simulées.

## Ce qui marche aussi

- **BMW et Mini à partir de la série F (environ 2008 et après)** : un [câble ENET](enet.md) atteint tous les calculateurs par Ethernet, ou un simple [ELM327](elm327-usb.md) avec l'adressage BMW sur CAN.
- **OBD standard sur les BMW avec CAN (environ 2007 et après)** : n'importe quel ELM327 lit les défauts moteur et les valeurs en direct.

## Appel à l'aide

Si vous avez un câble K+DCAN et une BMW plus ancienne, un [rapport de test](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) avec le journal aide à confirmer la partie K-Line sur une vraie voiture.
