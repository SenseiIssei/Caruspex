# DoIP (diagnostic sur IP)

Beaucoup de voitures à partir d'environ 2016 peuvent être diagnostiquées par Ethernet selon la norme ISO 13400, appelée DoIP. Un câble DoIP relie la prise OBD au port réseau de l'ordinateur.

## Connexion

1. Branchez le câble, mettez le contact et attendez que l'ordinateur ait son adresse automatique (169.254.x.x), ce qui peut prendre jusqu'à une minute.
2. Sur la page **Connecter**, choisissez **Câble réseau** et laissez le champ vide. Caruspex cherche les véhicules DoIP sur le réseau et reconnaît tout seul si la voiture parle DoIP ou le HSFZ de BMW.

## Ce qui fonctionne

L'OBD via DoIP, et les calculateurs via UDS dont Caruspex connaît les adresses pour la marque. Les listes d'adresses pour les voitures DoIP sont encore courtes ; les enregistrements de vraies voitures les complètent (voir [confidentialité](privacy.md) pour ce que contient un enregistrement partagé).

## Quel câble ?

Le DoIP a besoin des lignes Ethernet sur les broches 3, 11, 12 et 13 de la prise OBD et d'une ligne d'activation sur la broche 8. Les câbles vendus pour une marque précise sont généralement câblés exactement ainsi ; le câble ENET de BMW en fait partie.

## Ligne de commande

```
caruspex_cli enet
```

liste les véhicules BMW et DoIP trouvés sur le réseau.
