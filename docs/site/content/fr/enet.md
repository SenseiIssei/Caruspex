# Câble ENET BMW

Les BMW et Mini à partir de la série F (environ 2008 et après) ont de l'Ethernet sur la prise OBD. Un câble ENET va de la prise OBD au port réseau de l'ordinateur, sans puce d'adaptateur entre les deux. Caruspex parle HSFZ avec la passerelle de la voiture (ports 6811 et 6801) et atteint tous les calculateurs.

## Connexion

1. Branchez le câble sur la voiture et sur le port réseau de l'ordinateur (ou un adaptateur réseau USB). Mettez le contact.
2. Attendez jusqu'à une minute : sans serveur DHCP, Windows, macOS et Linux donnent tout seuls une adresse automatique à l'ordinateur (169.254.x.x).
3. Sur la page **Connecter**, choisissez **Câble réseau** et laissez le champ vide : Caruspex cherche la voiture sur le réseau. Si la recherche ne trouve rien, saisissez l'adresse de la voiture.

## Ce qui fonctionne

- Tous les calculateurs via UDS avec l'adressage BMW, codes défaut au format BMW tels qu'ISTA les affiche (par exemple 480A10).
- Mémoire de défauts, identification et effacement par calculateur, valeurs en direct via l'OBD.
- Ici aussi, Caruspex ne fait que lire. Le codage et la programmation restent l'affaire des logiciels d'atelier.

## Quel câble ?

Un câble ENET classique a une résistance entre les broches 8 et 16 qui active l'Ethernet de la voiture. Les câbles sans elle restent muets. Les ports réseau à 1 Gbit/s fonctionnent, la voiture parle à 100 Mbit/s.

## Ligne de commande

```
caruspex_cli ecus --enet auto
```
