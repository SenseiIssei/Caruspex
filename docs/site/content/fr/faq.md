# Questions fréquentes

## Caruspex est-il payant ?

Non. Caruspex est un logiciel libre sous licence MIT, sans publicité, sans compte et sans options payantes.

## Quel adaptateur acheter ?

Pour la plupart des gens, un ELM327 USB avec une puce FTDI ou CH340, ou un OBDLink (Bluetooth ou USB), est le meilleur départ. Pour VW, Audi, Seat et Skoda d'avant 2004 environ, ajoutez un [câble KKL](kkl.md) ; pour BMW et Mini à partir de la série F, un [câble ENET](enet.md). La [vue d'ensemble des interfaces](interfaces.md) les compare.

## Est-ce que ça marchera avec ma voiture ?

Toute voiture avec OBD-II ou EOBD fonctionne pour le moteur : en Europe, les voitures essence à partir de 2001 et les diesels à partir de 2004, aux États-Unis à partir de 1996. Les voitures plus anciennes du groupe VW à partir d'environ 1990 fonctionnent avec le câble KKL. La page **Véhicules** indique quelle méthode fonctionne pour une voiture donnée. Les motos à partir d'Euro 5 fonctionnent avec un câble adaptateur pour leur prise de diagnostic.

## A-t-il été testé sur ma voiture ?

Pour être honnête : pas encore. Caruspex est testé avec des calculateurs simulés ; sur du vrai matériel, seul un OBDLink avec la puce STN2232 a été vérifié jusqu'ici, sans voiture. Aidez-nous avec un [rapport de test](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Caruspex peut-il abîmer ma voiture ?

Caruspex ne fait que lire. La seule chose qu'il écrit, c'est l'effacement des codes défaut, et il prévient avant de le faire. Il ne code jamais, ne reprogramme jamais et ne modifie jamais les valeurs d'adaptation. Ne l'utilisez pas en roulant.

## Peut-il modifier le kilométrage, l'antidémarrage ou coder des fonctions ?

Non, et il ne le fera pas. C'est juridiquement délicat et ce n'est pas le but de cet outil.

## Pourquoi l'adaptateur annonce la version 2.1 alors que certaines choses échouent ?

Beaucoup d'adaptateurs bon marché annoncent une version d'ELM327 plus récente que ce qu'ils sont réellement. Le **Test de l'adaptateur** détermine quelles commandes l'adaptateur connaît vraiment.

## Pourquoi les valeurs sont-elles lentes sur ma voiture ancienne ?

Les voitures sur K-Line (environ 2001 à 2007 en Europe) répondent lentement par conception. Les voitures en CAN sont bien plus rapides.

## Caruspex envoie-t-il mes données quelque part ?

Non. Seule la recherche de mises à jour demande à GitHub la dernière version, et elle peut être désactivée. Voir [confidentialité](privacy.md).

## Comment puis-je aider ?

Envoyez un rapport de test de votre voiture, partagez un enregistrement, signalez des bugs ou améliorez une traduction : le guide est [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), et le bouton **Améliorer la traduction** dans les réglages l'ouvre.
