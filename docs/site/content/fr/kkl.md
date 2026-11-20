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

## Premier test sur la voiture

La page du mode expert **Premier test sur la voiture** vous guide pendant la première séance avec un nouveau câble KKL. Elle est adaptée à l'Audi A2 et fonctionne sur toute VW, Audi, Seat ou Skoda avec K-Line. Elle ne fait que lire : rien n'est effacé, pas de test des sorties, pas de réglage de base. Le port n'est ouvert qu'au lancement d'un test.

1. **Vérifiez le câble sans la voiture.** Branchez le câble seulement en USB, choisissez son port et appuyez sur **Vérifier le câble**. Caruspex lit la puce (FTDI, CH340, Prolific, CP210x), pour un FTDI le délai de latence, et mesure les bits d'une adresse à 5 bauds sur la ligne de break. Sans voiture, aucun écho ne revient, c'est normal.
2. **Réglez le délai de latence sur 1 ms** (câbles FTDI sous Windows) : Gestionnaire de périphériques, Ports (COM et LPT), double-clic sur USB Serial Port, Paramètres du port, Avancé, Latency Timer (msec) sur 1, OK, puis débranchez et rebranchez le câble une fois. Avec les 16 ms par défaut, la réponse aux octets de clé peut manquer sa fenêtre de 50 ms. Caruspex lit ce réglage, mais ne le modifie jamais.
3. **Sur la voiture :** contact mis, moteur arrêté, le câble bien enfoncé dans la prise OBD et branché directement sur l'ordinateur (pas de hub USB), aucun autre logiciel de diagnostic ouvert. Cochez la liste et appuyez sur **Lancer le premier test**.
4. Le test réveille le moteur (adresse 01) et mesure chaque bit de l'adresse, l'octet de synchronisation 0x55, les octets de clé et la réponse à ceux-ci. Il lit ensuite l'identification, la mémoire de défauts et les blocs de mesure 001 à 003, puis le combiné d'instruments 17, ABS 03, airbag 15, confort 46 et électricité centrale 09. Un calculateur qui ne répond pas n'est peut-être tout simplement pas monté.
5. **Enregistrer le rapport** écrit un fichier texte et un fichier JSON dans le dossier `test-reports` du dossier des paramètres de Caruspex. **Ouvrir le rapport de test sur GitHub** remplit le formulaire du rapport pour vous. Le VIN et le numéro d'antidémarrage sont raccourcis tant que la case n'est pas cochée.

Ce que signifient les résultats :

| Résultat | Que faire |
|---|---|
| Pas d'écho | Le câble ne s'entend pas lui-même : la fiche n'est pas bien en place, ou la broche 16 n'a pas ses 12 V (vérifiez le fusible de la prise diagnostic). Sans voiture, c'est normal. |
| L'écho est déformé | Un autre appareil sur la prise, l'interrupteur d'un câble K+DCAN pas sur K-Line, ou un câble défectueux. |
| L'adresse de réveil est cadencée de façon trop irrégulière | Les bits s'écartent de plus de 20 ms des 200 ms : pas de hub USB, fermez les autres programmes, délai de latence à 1 ms. Caruspex fait un nouvel essai avec les appels break anticipés. |
| Câble OK, mais pas de réponse de la voiture | Le contact est-il mis ? L'adresse 01 est le moteur. Les voitures à partir de 2005 environ se diagnostiquent par CAN. |
| La réponse aux octets de clé est arrivée trop tard | Réglez le délai de latence sur 1 ms. |

## Problèmes fréquents

- **Pas de réponse du calculateur** : contact mis ? Le réveil à 5 bauds demande un timing exact ; les hubs USB et certaines machines virtuelles le perturbent, branchez le câble directement.
- **Coupe au bout d'un moment** : les câbles bon marché perdent des octets. Essayez un autre câble avant de soupçonner la voiture.
