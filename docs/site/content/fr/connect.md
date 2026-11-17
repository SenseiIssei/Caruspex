# Assistant de connexion et prise OBD

## Où se trouve la prise ?

Toute voiture vendue en Europe depuis 2001 (essence) ou 2004 (diesel), et aux États-Unis depuis 1996, a une prise OBD à 16 broches à portée de main du conducteur. Emplacements habituels :

- sous la planche de bord, à gauche ou à droite de la colonne de direction,
- au-dessus des pédales,
- dans la garniture latérale du plancher,
- derrière le couvercle de la boîte à fusibles,
- dans la console centrale, parfois derrière un petit cache.

Pour plus de 200 générations de voitures et 35 motos, Caruspex connaît l'emplacement exact. L'assistant de connexion le montre sur un plan vu de dessus et sur une vue rapprochée du côté conducteur ; pour les motos, il affiche une vue de côté avec la prise de diagnostic et le câble adaptateur nécessaire.

> La liste des emplacements de prise est notre propre collecte et n'a pas été vérifiée sur toutes les voitures. Si l'emplacement est faux pour la vôtre, dites-le-nous avec un [rapport de test](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## L'assistant de connexion

L'assistant s'ouvre tout seul au premier démarrage. Il pose quatre questions, et chaque étape peut être sautée :

1. **Quel adaptateur ?** ELM327 en USB, Bluetooth ou Wi-Fi, un câble ENET ou un câble KKL. Des images aident à les reconnaître.
2. **Quelle voiture ?** Choisissez-la dans le catalogue, l'assistant montre alors où se trouve la prise.
3. **Contact mis.** La plupart des calculateurs ne répondent que contact mis.
4. **Connecter.** Caruspex parcourt les ports et essaie les protocoles.

Par la suite, vous trouvez l'assistant dans les réglages et sur la page Connecter.

## Recherche et mémorisation

Avec la recherche automatique, Caruspex passe en revue tous les ports série, cherche un ELM327 sur chacun, puis essaie un par un les protocoles de la voiture. Une fois une voiture connectée, Caruspex retient le chemin (adaptateur, port, protocole, débit) pour son VIN et l'essaie en premier la fois suivante, ce qui accélère la connexion.

## Quand la connexion ne marche pas

Si aucun adaptateur ne répond ou si la voiture reste muette, Caruspex affiche une courte liste de contrôle pour ce type de connexion, avec ce qu'il a déjà constaté : pilote installé, Bluetooth appairé, contact mis, bon débit, un adaptateur clone qui se fait seulement passer pour une version plus récente. Les pages de la rubrique [adaptateurs et interfaces](interfaces.md) expliquent chaque type en détail.
