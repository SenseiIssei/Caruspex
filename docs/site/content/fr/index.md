# Caruspex

Caruspex est un logiciel de diagnostic gratuit pour voitures et motos. Il tourne sous Windows, Linux et macOS, parle 18 langues et fonctionne avec les adaptateurs que les gens possèdent vraiment : la fiche ELM327 bon marché, un OBDLink, un câble KKL pour les voitures plus anciennes du groupe Volkswagen, un câble ENET pour BMW, et des interfaces CAN professionnelles.

![La vue d'ensemble avec les cadrans en direct](screenshot-overview.png)

## Ce qu'il fait

- Lit et efface les codes défaut, les explique avec des mots simples et dit si vous pouvez continuer à rouler.
- Affiche les valeurs en direct : températures, vitesse, mélange, pression de suralimentation, tension de batterie et bien d'autres, en chiffres et en courbes.
- Interroge chaque calculateur de la voiture, pas seulement le moteur, dans la mesure où la voiture et l'adaptateur le permettent.
- Contrôle une voiture d'occasion avant l'achat : le VIN correspond-il, les kilométrages concordent-ils, des défauts ont-ils été effacés peu avant ?
- Montre la voiture comme un modèle en verre dans lequel défauts et valeurs s'allument là où ils se trouvent.
- Vous dit quoi faire ensuite : s'arrêter maintenant, aller au garage bientôt, ou ça peut attendre.

## Par où commencer

1. [Installez Caruspex](install.md) sur votre ordinateur.
2. Suivez les [premiers pas](first-steps.md) : essayez la voiture de démo, puis connectez la vôtre.
3. Cherchez votre adaptateur dans [adaptateurs et interfaces](interfaces.md).
4. Lisez ce que Caruspex enregistre et envoie dans [confidentialité](privacy.md). En bref : rien ne quitte votre ordinateur, sauf si vous le partagez vous-même.

> Caruspex ne fait que lire. Il ne code jamais les calculateurs, ne modifie jamais le kilométrage et ne touche jamais à l'antidémarrage. Effacer les codes défaut est la seule chose qu'il écrit dans la voiture.

## État honnête

Caruspex est développé d'après les spécifications et testé avec des calculateurs simulés, de fausses passerelles et de faux pilotes. Sur du vrai matériel, seul un OBDLink avec la puce STN2232 a été vérifié jusqu'ici, sans voiture. Si vous essayez Caruspex sur votre voiture, envoyez un [rapport de test](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) : chaque rapport allonge la liste des voitures testées.
