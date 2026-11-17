# ELM327 en Wi-Fi

Certains adaptateurs ouvrent leur propre réseau Wi-Fi. L'ordinateur s'y connecte et communique avec l'adaptateur via TCP.

## Connexion

1. Branchez l'adaptateur et mettez le contact. Au bout de quelques secondes, un réseau Wi-Fi apparaît, souvent nommé `WiFi_OBDII`, `V-LINK` ou d'après le fabricant.
2. Connectez votre ordinateur à ce réseau. Il n'a pas d'accès à internet, votre système peut donc signaler une connexion limitée : c'est normal.
3. Dans Caruspex, choisissez **Wi-Fi** sur la page **Connecter**. La plupart des adaptateurs utilisent l'adresse `192.168.0.10` et le port `35000` ; c'est préréglé. Si le vôtre en utilise une autre (par exemple `192.168.4.1`), saisissez-la sous la forme `address:port`.

## Conseils

- Tant que l'ordinateur est sur le réseau de l'adaptateur, il n'a pas internet. Cela ne gêne pas Caruspex.
- Les adaptateurs Wi-Fi fonctionnent sur tous les systèmes sans pilote.
- Si la connexion coupe, rapprochez l'ordinateur de la voiture : la radio de ces boîtiers est faible.
