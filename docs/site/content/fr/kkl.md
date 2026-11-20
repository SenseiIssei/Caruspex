# Câble KKL (VAG 409.1)

Un câble KKL est un câble USB avec une puce série et un pilote de ligne K, sans ELM327 à l'intérieur. Caruspex pilote lui-même la K-Line, octet par octet. C'est ce dont ont besoin les VW, Audi, Seat et Skoda plus anciennes : leur protocole KWP1281 acquitte chaque octet, ce qu'aucun ELM327 ne sait faire.

## Ce qui fonctionne

- Tous les calculateurs de VW, Audi, Seat et Skoda d'environ 1990 à 2004 : moteur, boîte de vitesses, ABS, airbag, combiné d'instruments, confort et d'autres.
- KWP1281 et KWP2000 (ISO 14230), détectés automatiquement d'après les octets clés après le réveil.
- Mémoire de défauts par calculateur, blocs de mesure, et le vrai réglage de base du boîtier papillon (bloc de mesure 060 ou 098).
- Les canaux d'adaptation sont volontairement en lecture seule. Le test des sorties actionne de vraies pièces et ne démarre qu'après votre confirmation.

Ces pages se trouvent dans le **mode expert** : **Calculateurs via KKL**, **Boîtier papillon** et **Adaptation et tests des sorties**. Chacune ouvre elle-même le câble : choisissez le port série, le calculateur, lancez.

## Quel câble ?

Les câbles avec une vraie puce FTDI FT232RL ou une CP2102 sont bien plus fiables sur la K-Line que les clones les moins chers. Pour le KWP1281, une vraie FTDI est recommandée. Le câble doit laisser passer la K-Line telle quelle : les interfaces "HEX-USB" avec leur propre firmware ne fonctionnent pas.

Les voitures d'avant 1996 environ n'ont souvent pas de prise OBD mais une prise 2x2 dans le compartiment moteur ou sous la planche de bord. Un adaptateur 2x2 vers la prise à 16 broches ne coûte pas cher.

## Vérifier le câble

L'outil en ligne de commande teste le câble sans voiture :

```
caruspex_cli kklcheck --port COM3
```

## Problèmes fréquents

- **Pas de réponse du calculateur** : contact mis ? Le réveil à 5 bauds demande un timing exact ; les hubs USB et certaines machines virtuelles le perturbent, branchez le câble directement.
- **Coupe au bout d'un moment** : les câbles bon marché perdent des octets. Essayez un autre câble avant de soupçonner la voiture.
