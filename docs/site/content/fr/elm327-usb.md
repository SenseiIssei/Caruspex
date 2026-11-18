# ELM327 en USB

Une fiche pour la prise OBD avec un câble vers le port USB. C'est le type d'ELM327 le plus fiable : pas d'appairage, pas de liaison radio, pas de piles à plat.

## Pilote

Dans la fiche, une puce USB transforme le câble en port série (port COM sous Windows). Le pilote nécessaire dépend de cette puce :

| Puce | Pilote |
|---|---|
| FTDI FT232 | généralement installé par Windows lui-même, intégré à Linux et macOS |
| CH340 / CH341 | généralement installé par Windows lui-même, intégré à Linux et macOS |
| Silicon Labs CP210x | sur le site du fabricant si Windows ne le trouve pas |
| Prolific PL2303 | sur le site du fabricant ; les vieux clones ne fonctionnent souvent qu'avec un ancien pilote |

Une fois branché, le Gestionnaire de périphériques de Windows affiche l'adaptateur sous "Ports (COM et LPT)".

## Connexion

1. Branchez l'adaptateur sur la voiture, mettez le contact, branchez le câble sur l'ordinateur.
2. Sur la page **Connecter**, choisissez **Port USB ou COM**.
3. Laissez le port sur **Recherche automatique** ou choisissez-le dans la liste, puis connectez.

La recherche automatique essaie chaque port série aux vitesses habituelles. Si vous connaissez la vitesse de votre adaptateur, vous pouvez la régler dans les réglages sous **Vitesse de transmission de l'adaptateur**.

## Linux et macOS

Sous Linux, l'adaptateur apparaît comme `/dev/ttyUSB0` ou `/dev/ttyACM0`. Votre utilisateur doit faire partie du groupe `dialout` (voir [Installation](install.md)). Sous macOS, il apparaît comme `/dev/cu.usbserial-...` ou `/dev/cu.wchusbserial...`.

## Problèmes fréquents

- **Aucun port n'apparaît** : le pilote manque ou le câble est un simple câble de charge.
- **L'adaptateur répond, la voiture non** : contact coupé, ou l'adaptateur est un clone qui ne gère pas le protocole de la voiture. Le [test de l'adaptateur](interfaces.md) vous en dit plus.
- **Valeurs en direct très lentes sur les voitures anciennes** : la K-Line elle-même est lente, ce n'est pas la faute de l'adaptateur.
