# Simulateur de tuning

La page **Simulateur de tuning** (mode pro, sous Mesurer) est une BMW N55 virtuelle : le six cylindres en ligne de 3,0 litres avec un turbo twin-scroll de la F30 335i et de la F32/F36 435i, 306 ch et 400 Nm. Modifiez la pression de suralimentation, l'allumage et le lambda par plage de régime et voyez ce qu'en dit le modèle, bien avant que quoi que ce soit ne touche une voiture.

> **Simulation uniquement.** Caruspex n'écrit jamais de cartographie moteur et ne reprogramme jamais. Il ne connaît aucun format de fichier de flash, aucune somme de contrôle ni adresse de cartographie, et ne copie aucune préparation commerciale. Le vrai tuning relève d'outils éprouvés et d'un atelier, et la modification doit être homologuée (en Allemagne par le TÜV). Sans cela, la voiture perd son homologation.

![Simulateur de tuning](screenshot-tune.png)

## Ce que vous pouvez modifier

- Des écarts dans cinq plages de régime (1000 à 2000, 2000 à 3500, 3500 à 5000, 5000 à 6000, 6000 à 7000) : pression en bar, allumage en degrés, lambda.
- Le carburant (RON 95, 98, 100 ou 102), la température extérieure, la pression atmosphérique et la boîte (automatique 8HP45 ou manuelle à six rapports).
- **Origine** remet les écarts à zéro, **Réinitialiser** aussi les conditions.

La page montre couple et puissance d'origine face à la version modifiée, chaque limite touchée avec sa raison, et les valeurs internes : pression, air de suralimentation, allumage, réserve de cliquetis, température d'échappement, turbo, pompe haute pression, pression moyenne effective.

## Comment le modèle calcule

1. **Air.** La consigne de pression est limitée par ce que le turbo peut produire à ce régime, par son rapport de pression et par son débit. Le compresseur chauffe l'air, l'échangeur en retire une partie. Pression, température et remplissage donnent la masse d'air.
2. **Carburant et allumage.** Le carburant découle de l'air et du lambda. L'allumage est l'avance d'origine plus votre écart, mais jamais plus tôt que ne le permet la limite de cliquetis : la régulation de cliquetis du moteur le retirerait. Cette limite bouge avec l'indice d'octane, la température de l'air, la pression et le lambda.
3. **Puissance.** L'énergie du carburant fois un rendement qui baisse avec l'écart à la meilleure avance, moins les frottements, donne couple et puissance.
4. **Limites.** Turbo, température d'échappement, pompe haute pression, injecteurs, pré-allumage à bas régime, boîte, air de suralimentation et mélange donnent chacun un avertissement en mots simples.

Une courbe complète (25 points de 1000 à 7000 tr/min) prend environ 0,015 ms dans la version publiée.

## Données publiées

| Valeur | Chiffre | Source |
|---|---|---|
| Cylindrée, alésage, course, compression | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Couple | 400 Nm de 1200 à 5000 tr/min | Wikipedia "BMW N55", données BMW de la 435i |
| Puissance | 225 kW (306 ch) de 5700 à 6000 tr/min | Wikipedia (5700 à 5800), BMW (5800 à 6000) |
| Régime maximal | 7000 tr/min | Wikipedia "BMW N55" |
| Couple d'entrée nominal de la 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Hypothèses

Personne ne publie sous licence libre les cartographies internes du N55. Ces valeurs sont mes hypothèses ; chacune est justifiée dans [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- La **pression d'origine** est recalculée avec le modèle à partir de la courbe publiée (environ 0,7 à 0,8 bar jusqu'à 5000 tr/min, moins au-delà). Que la courbe d'origine corresponde montre donc la cohérence, pas la vérité.
- Remplissage 0,86 à 0,97, rendement indiqué 0,39, frottements quadratiques selon le régime.
- Allumage limité par le cliquetis de 3 à 15 degrés avant le point mort haut avec 1,5 degré de réserve ; il bouge de 1 degré par point d'octane, 0,1 degré par kelvin d'air, 6 degrés par bar et 2,5 degrés par 0,1 de lambda.
- Turbo : rapport de pression jusqu'à 2,55, débit corrigé jusqu'à 0,32 kg/s, rendement 0,72. Efficacité de l'échangeur 0,85, moins à fort débit.
- Limite de température d'échappement 980 °C, pompe haute pression 0,027 kg/s, fenêtre d'injection de 40 % du cycle, risque de pré-allumage au-dessus de 18 bar de pression moyenne effective sous 2500 tr/min, boîte manuelle environ 450 Nm.
- Carburant E5 : rapport air carburant 14,4, 42,4 MJ/kg.

## Quelle précision ?

- Origine face à la courbe publiée : 400 Nm à 1 % près de 1250 à 5000 tr/min, 306 ch à 6000 tr/min et 304 ch à 5750 tr/min. Les tests tolèrent 3 %.
- Le modèle n'a encore été vérifié sur aucun vrai N55. Chaque chiffre est une estimation : environ ±8 % pour le couple et la puissance, ±0,1 bar de pression, ±10 K d'air de suralimentation, ±3 degrés de réserve de cliquetis, ±60 K de température d'échappement.

## Comparer avec votre voiture

1. Connectez-vous, ouvrez **Valeurs en direct** avec régime, charge, pression d'admission, température d'air d'admission, avance à l'allumage, lambda et masse d'air, et lancez un enregistrement.
2. Accélérez une fois à fond sur toute la plage de régime, seulement là où c'est autorisé et sûr.
3. Ouvrez l'enregistrement sur la page du simulateur. Caruspex garde les points en pleine charge, les regroupe par plages de 500 tr/min et montre mesuré face à simulé : pression d'admission et air de suralimentation avec vos cartographies, masse d'air et allumage avec la pression et la température mesurées.

L'OBD ne transmet la pression d'admission que jusqu'à 255 kPa, et le capteur que le calculateur utilise pour la température d'admission n'est pas documenté. **Voir un exemple** remplit le tableau de valeurs inventées pour montrer comment il se lit.

## Bon à savoir

- Plus de couple que d'origine à bas régime augmente le risque de pré-allumage, qui peut casser des pistons d'un coup.
- Une avance plus forte que la limite de cliquetis n'apporte rien : la régulation de cliquetis la retire.
- Un meilleur carburant seul donne de la réserve, pas de puissance : la cartographie d'allumage d'origine est le plafond.
