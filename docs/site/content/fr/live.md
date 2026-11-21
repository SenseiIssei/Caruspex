# Valeurs en direct

La page **Valeurs en direct** (mode expert) montre ce que mesurent les capteurs en ce moment. En mode débutant, le **Mode conduite** affiche les valeurs les plus importantes en gros caractères.

![Valeurs en direct](screenshot-live.png)

## Ce qu'il y a

- Toutes les valeurs du mode 01 de la SAE J1979 jusqu'au PID C4 : 145 PID avec plus de 300 mesures, regroupées et consultables par recherche. C'est la voiture elle-même qui dit lesquelles elle propose.
- Kilométrage, rapport engagé, température d'huile, consommation, couple, pression de suralimentation, températures d'échappement, filtre à particules, AdBlue, NOx et batterie hybride, dans la mesure où la voiture les transmet.
- Une courbe défilante pour plusieurs valeurs à la fois. Sur de longues fenêtres (jusqu'à une heure), la courbe éclaircit les anciens points mais garde chaque pic et chaque creux.
- Enregistrement en CSV.

## Valeurs constructeur et valeurs personnelles

- **Batteries de voitures électriques** : état de santé, charge, tensions des cellules avec leur écart et températures pour Hyundai Kona et Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf et Tesla Model 3 et Y (par écoute du bus CAN), sur la page **Batterie VE**.
- **Valeurs OBDb** pour environ 550 véhicules de plus (pression des pneus, huile, kilométrage et d'autres), proposées d'elles-mêmes pour la voiture détectée.
- **Session de diagnostic étendue** : certaines valeurs des diesels du groupe Volkswagen, comme la masse de suie mesurée et la distance depuis la dernière régénération, ne sont fournies par le calculateur moteur que dans sa session de diagnostic étendue. En mode expert, Caruspex y fait passer le calculateur tant que ces valeurs sont interrogées, uniquement pour les lire, les marque avec **Session 03** et le remet ensuite en session normale. Cela se désactive dans les réglages, sous **Valeurs constructeur**. Si le calculateur refuse, par exemple en roulant, la valeur en indique la raison.
- **Vos propres valeurs** au format CSV de Torque, très répandu, équations comprises.

![Batterie VE](screenshot-battery.png)

## À quelle vitesse ?

Le nombre de valeurs par seconde dépend de la voiture et de l'adaptateur. Les voitures en CAN répondent vite ; les voitures sur K-Line sont lentes par conception. Un OBDLink permet à Caruspex de demander jusqu'à 24 valeurs d'un coup (voir [OBDLink et STN](obdlink-stn.md)) ; une interface CAN brute est le moyen le plus rapide.

## Enregistrement et relecture

Une session entière avec la voiture peut être enregistrée dans un fichier et relue plus tard sans la voiture, pratique pour chercher une panne. Avec **Partager l'enregistrement**, un enregistrement peut être préparé pour le projet sans données personnelles, voir [confidentialité](privacy.md).
