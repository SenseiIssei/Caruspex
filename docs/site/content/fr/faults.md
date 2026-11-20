# Défauts

La page **Mémoire de défauts** lit ce que les calculateurs ont enregistré et l'explique avec des mots simples.

![Mémoire de défauts](screenshot-faults.png)

## Ce que vous voyez

- Les codes **enregistrés, en attente et permanents**. Avec plusieurs calculateurs, chaque code indique qui l'a signalé (moteur, boîte de vitesses).
- Environ 3 400 codes standard (P, C, B, U) et les codes constructeur de VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai et Kia, Subaru et d'autres. Un code P1 est interprété comme l'entend la marque détectée.
- Un **feu tricolore** par code et la question que tout le monde se pose en premier : puis-je continuer à rouler ?
- Les causes habituelles, ce que vous pouvez vérifier vous-même et une fourchette de coût approximative.
- L'emplacement de la pièce : la voiture en transparence, ou la [vue radiographique](xray.md).

## Tous les calculateurs

En mode expert, la page **Tous les calculateurs** interroge chaque calculateur, pas seulement le moteur : ABS, airbag, combiné d'instruments, climatisation et d'autres. Chaque défaut est accompagné de son type de panne (court-circuit, circuit ouvert, signal non plausible), de la référence de pièce et de la version logicielle. Les voitures plus anciennes du groupe VW se lisent avec un [câble KKL](kkl.md).

## Que dois-je faire maintenant ?

La page **Que faire ?** classe tout ce que Caruspex a trouvé en trois niveaux : s'arrêter maintenant, au garage bientôt, peut attendre. Chaque point a une phrase simple qui dit pourquoi, une fourchette de coût approximative et une note à montrer au garage. L'OBD ne peut pas voir si le voyant moteur clignote ou si un témoin rouge est allumé, deux cases vous le demandent donc ; un voyant qui clignote avec des ratés d'allumage fait passer le point à "s'arrêter maintenant".

![Que dois-je faire maintenant](screenshot-todo.png)

## Effacer les codes

L'effacement se fait par calculateur, avec un avertissement clair, et Caruspex relit la mémoire juste après.

> Effacer n'est pas réparer. Cela remet aussi à zéro les moniteurs de disponibilité : tant que la voiture n'a pas fait plusieurs cycles de conduite complets, elle échoue au contrôle antipollution, et un code dont la cause est toujours là revient après un court trajet.

## Export

La liste peut être enregistrée en CSV, et le rapport d'inspection en PDF.
