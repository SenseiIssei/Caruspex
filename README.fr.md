# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · **Français** · [Português](README.pt.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Diagnostic automobile open source pour Windows, Linux et macOS, écrit en Rust avec une interface Tauri 2.
Un câble ou un adaptateur, une application : lire et effacer les codes défaut, suivre les données des capteurs en direct,
vérifier l'état de préparation pour les émissions, et dialoguer octet par octet avec les calculateurs VW/Audi par un câble K-Line.

Le nom vient de l'*haruspice*, le devin romain qui lisait les présages dans les entrailles. Caruspex lit celles de la voiture.

L'application parle 18 langues, aussi bien l'interface que chaque texte de code défaut : anglais, allemand, espagnol,
français, portugais, italien, néerlandais, polonais, tchèque, russe, ukrainien, turc, arabe (de droite à gauche), chinois,
japonais, coréen, hindi et indonésien. Elle suit la langue du système, peut être changée dans les réglages, ou
forcée à chaque lancement avec `--language fr` et ainsi de suite. La façon dont les traductions sont faites et vérifiées est décrite dans
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Vue d'ensemble avec jauges en direct](docs/screenshot-overview.png)

## Points forts

- **Deux piles de diagnostic complètes** : OBD-II/EOBD standard avec n'importe quel adaptateur ELM327, et diagnostic
  constructeur VW/Audi (KWP1281 et KWP2000) par un câble KKL K-Line brut.
- **Protocoles implémentés de zéro** : KWP1281 acquitte chaque octet avec son complément à un,
  ce que le micrologiciel de l'ELM327 ne sait pas faire. Caruspex pilote lui-même la K-Line, y compris l'adresse de
  réveil à 5 bauds, émise bit par bit en basculant le signal break du port série avec une temporisation fondée sur des échéances, de sorte que
  la gigue de l'ordonnancement du système d'exploitation ne s'accumule pas sur les dix bits.
- **Détection automatique du protocole** : après le réveil, les deux octets de clé du calculateur indiquent s'il parle
  KWP1281 ou KWP2000 (ISO 14230) ; Caruspex bascule en conséquence.
- **Espace de travail Rust modulaire** : un crate par domaine (liaisons avec l'adaptateur, protocoles, réseau et CAN, véhicules, données,
  enregistrements, analyse, voiture de démo, outil en ligne de commande), et l'application Tauri par-dessus. Le cœur ne connaît rien
  de l'interface, donc l'application et l'outil en ligne de commande exécutent exactement le même code.
- **Testé sans voiture** : des calculateurs simulés (`crates/caruspex-protocols/tests/common/`) reproduisent la
  poignée de main octet par octet, de fausses passerelles tiennent lieu d'ENET et de DoIP, un faux pilote J2534 couvre le chemin PassThru,
  et un simulateur ELM327 en Python permet des essais complets de l'application.
- **Un seul jeu de traductions** : le cœur et l'interface partagent les mêmes fichiers de traduction dans
  `translations/<code>/`, en 18 langues. Le cœur retrouve ses textes grâce au texte anglais, l'interface construit
  ses catalogues à partir des mêmes fichiers.
- **Reconnaît toutes les voitures** : un décodeur de VIN avec 365 codes constructeur et un catalogue de plus de 2 100
  générations de modèles depuis 1980 indiquent quelle voiture est connectée et ce qui fonctionne avec elle.
- **Voiture de démo intégrée** : un ELM327 dans le même processus avec un moteur et une boîte de vitesses sur CAN, pour tout essayer
  sans matériel.
- **Données générées, pas saisies à la main** : environ 1 700 codes défaut, dont beaucoup calculés à partir de familles de codes régulières
  (par cylindre, par rangée, par sonde lambda) par `tools/gen_dtc.py`.

## Fonctions

### Connaît la voiture
- Après la connexion, Caruspex lit le VIN et vous dit de quelle voiture il s'agit : marque, modèle, génération, année modèle.
- Un catalogue de plus de 2 100 générations de modèles de 94 marques, de 1980 à aujourd'hui, y compris des marques qui n'existent plus.
- Pour chaque génération : quelle prise de diagnostic, où elle se trouve, quel protocole, et si l'OBD-II, un câble KKL ou seulement les outils du constructeur fonctionneront.
- Un vérificateur de VIN pour n'importe quel VIN : constructeur, pays, année modèle, chiffre de contrôle, code du modèle.
- Un garage qui reconnaît chaque voiture à son VIN et conserve l'historique de ses contrôles et de ses relevés de compteur kilométrique.

![Véhicules](docs/screenshot-vehicles.png)

### Bilan de santé en un bouton
- Lit la mémoire de défauts, les moniteurs de disponibilité et les tests de composants, puis surveille le mélange, les températures, les sondes lambda et la tension de charge pendant une demi-minute.
- Une note de 0 à 100 où chaque point perdu est expliqué en termes simples, avec la prochaine étape.
- Prêt pour le contrôle antipollution ? Quel moniteur reste ouvert et comment rouler pour le terminer.
- Un rapport d'inspection en PDF, à imprimer ou à remettre à l'atelier.

![Bilan de santé](docs/screenshot-health.png)

### Mémoire de défauts
- Codes enregistrés, en attente et permanents ; avec plusieurs calculateurs, chaque code indique qui l'a signalé (moteur, boîte de vitesses).
- Environ 1 400 codes standard (P, C, B, U) plus des codes constructeur pour VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru et d'autres. Un code P1 est lu comme l'entend la marque détectée.
- Un feu tricolore par code et la question que tout le monde pose d'abord : puis-je continuer à rouler ?
- Causes habituelles, points que vous pouvez vérifier vous-même et une classe de coût approximative.
- La voiture en vue transparente montre où se trouve la pièce concernée.
- Effacement avec un avertissement clair et relecture automatique ; export CSV.

![Mémoire de défauts](docs/screenshot-faults.png)

### Que dois-je faire maintenant ?
- Une page qui classe tout ce que Caruspex a trouvé en trois étapes : s'arrêter tout de suite, au garage bientôt, peut attendre.
- Chaque point avec une phrase simple qui explique pourquoi, une fourchette de coût approximative et une note à montrer au garage (code, calculateur, zone, causes habituelles).
- L'OBD ne peut pas voir si le voyant moteur clignote ou si un voyant rouge est allumé : deux cases à cocher posent la question, et un voyant qui clignote avec des ratés d'allumage fait passer le point en "s'arrêter tout de suite".

![Que dois-je faire maintenant](docs/screenshot-todo.png)

### Vérifier une voiture d'occasion
- Pour la visite avant l'achat : le VIN de la voiture correspond-il aux papiers et au catalogue, le kilométrage concorde-t-il entre les calculateurs, les défauts ont-ils été effacés peu avant (distance et durée depuis l'effacement) ?
- Moniteurs de disponibilité, défauts mémorisés, le bilan de santé et, sur les voitures électriques, l'état de la batterie.
- Un feu tricolore, les questions à poser au vendeur et un PDF pour l'achat.

![Vérifier une voiture d'occasion](docs/screenshot-used-car.png)

### Vue radiographique en 3D
- La voiture connectée sous forme de maquette en verre que l'on peut tourner : moteur ou moteurs, boîte de vitesses, réservoir ou batterie haute tension, échappement, radiateur, batterie 12 V et prise OBD, disposés selon la carrosserie et la motorisation de la voiture détectée.
- Chaque calculateur trouvé par l'analyse se trouve là où il est d'habitude, relié par ses lignes de données (CAN groupe motopropulseur et confort) ; pendant que Caruspex lit, de petits paquets voyagent vers le calculateur interrogé.
- Les défauts éclairent leur zone et colorent en rouge le calculateur concerné, les valeurs en direct se posent sur la pièce à laquelle elles appartiennent et "Vérifier la voiture" amène la caméra vers chaque constat. Les couches peuvent être désactivées une à une.
- Des dimensions typiques pour la carrosserie et l'époque, pas une réplique du modèle exact : les données de conception et les schémas électriques des constructeurs ne sont pas librement accessibles.

![Vue radiographique](docs/screenshot-xray.png)

### Valeurs en direct
- Toutes les valeurs du mode 01 de la SAE J1979 jusqu'au PID C4 : 145 PID avec plus de 300 mesures, regroupées et consultables.
- Compteur kilométrique, rapport engagé, température d'huile, débit de carburant, couple, pression de suralimentation, températures d'échappement, filtre à particules, AdBlue, NOx et batterie hybride, dans la mesure où la voiture les communique.
- Graphique défilant et enregistrement CSV.

![Valeurs en direct](docs/screenshot-live.png)

### Mesurer
- Ordinateur de bord : consommation en direct et moyenne, coût, CO2, distance et note de conduite souple, même pour les voitures sans ordinateur de bord propre.
- Performances : 0 à 100 km/h, 80 à 120, 400 mètres départ arrêté (quart de mile) et distance de freinage à partir du signal de vitesse (routes fermées uniquement).
- Batterie et alternateur : tension au repos, chute au démarrage, tension de charge avec et sans charge, guidé en quatre étapes.

### Inspection
- Moniteurs de disponibilité en termes simples, image figée, VIN.
- Surveillance des composants du mode 06 avec valeur et limites, nombre de ratés d'allumage par cylindre.
- Noms des calculateurs, ID de calibration et sommes de contrôle du mode 09, par calculateur.
- Terminal ELM327 brut.

### Contrôle de l'adaptateur
- ELM327 d'origine ou clone ? Il demande les commandes de chaque version, mesure le temps de réponse et dit honnêtement ce qu'on peut en conclure.
- Reconnaît OBDLink et d'autres adaptateurs STN.
- N'écrit jamais rien dans l'adaptateur.

![Contrôle de l'adaptateur](docs/screenshot-adapter.png)

### Tous les câbles, toutes les voies d'accès à la voiture
Caruspex détecte tout seul ce qui est branché et ce que parle la voiture.

| Interface | Exemples | Comment |
|---|---|---|
| ELM327 par USB, Bluetooth, Bluetooth LE, Wi-Fi | ELM327 bon marché, Vgate, vLinker, Veepeak | port série, appairé ou BLE direct, TCP |
| Puces STN | OBDLink SX, EX, MX+, CX | comme ELM327, avec une temporisation plus rapide |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A en mode J2534 | le pilote 64 bits du constructeur, CAN avec ISO-TP géré par Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | port série, trouvé par la recherche automatique de ports |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable avec candleLight, cartes CAN pour Raspberry Pi | can0 et compagnie |
| Câble KKL (VAG 409.1) | câbles FTDI ou CP2102 | K-Line, KWP1281 et KWP2000 |
| Câble ENET | BMW et Mini à partir de la série F | HSFZ sur Ethernet |
| Câble DoIP | de nombreuses voitures à partir de 2016 environ | ISO 13400 sur Ethernet |

Sur chacune d'elles, les mêmes fonctions marchent : OBD-II et OBD sur UDS, tous les calculateurs sur UDS, adressage
BMW, écoute des valeurs diffusées (Tesla), enregistrement et relecture.

![Connecter](docs/screenshot-connect.png)

### Tous les calculateurs sur CAN (UDS)
- Avec un ELM327 ordinaire, Caruspex interroge chaque calculateur l'un après l'autre : moteur, boîte de vitesses, ABS, airbag, combiné d'instruments, climatisation et plus, sur la plupart des voitures à partir de 2008 environ.
- Mémoire de défauts par calculateur avec le type de panne (court-circuit, circuit ouvert, non plausible ...), référence de la pièce, version logicielle et matérielle.
- Listes d'adresses pour VAG, BMW et Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault et PSA ; pour tout le reste, un balayage des adresses usuelles.
- BMW et Mini à partir de la série F avec l'adressage étendu propre à BMW sur 6F1, plus un balayage des 128 adresses de diagnostic ; codes défaut au format BMW tels qu'ISTA les affiche (p. ex. 480A10).
- Effacement par calculateur, avec un avertissement.

![Tous les calculateurs](docs/screenshot-ecus.png)

### VW et Audi par un câble KKL (KWP1281 et KWP2000)
- Adaptation du boîtier papillon : le véritable réglage de base dans le bloc de mesure 060 ou 098.
- Analyse de tous les calculateurs, y compris ABS, airbag, combiné d'instruments et confort, avec un rapport imprimable.
- Canaux d'adaptation et tests des sorties, volontairement en lecture seule.

### Valeurs constructeur et personnalisées
- Batteries de voitures électriques : état de santé, charge, tensions des cellules avec écart, températures, pour Hyundai Kona et Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf et Tesla Model 3 et Y (en écoutant le bus CAN) ; chaque valeur avec une source documentée.
- Valeurs constructeur pour environ 550 véhicules de plus, issues du projet [OBDb](https://github.com/OBDb) (batterie, pression des pneus, huile, compteur kilométrique et plus), proposées automatiquement pour la voiture détectée ; chaque équation est vérifiée avec le décodeur d'OBDb lui-même.
- Chargez vos propres valeurs au format CSV de Torque, très répandu, avec un évaluateur complet des équations de Torque.

![Batterie VE](docs/screenshot-battery.png)

### Enregistrer et rejouer
- Enregistrez une session avec la voiture dans un fichier et rejouez-la plus tard sans la voiture, pratique pour chercher une panne.

### Sans voiture ni adaptateur
- Une voiture de démo intégrée : elle roule, chauffe, a quelques défauts enregistrés et parle comme un vrai ELM327 sur CAN.
- Un lexique hors ligne de tous les codes, filtrable par marque.

### Aspect et ergonomie
- Une nouvelle interface dans le style d'Odysync : calme, sombre, avec des transitions fluides.
- Connexion pas à pas avec une recherche automatique sur tous les ports et, en cas d'échec, une courte liste de contrôle pour ce type de connexion (pilote, appairage, contact, débit, adaptateurs clones) avec ce que Caruspex a déjà découvert.
- Mode débutant et expert : les débutants voient les pages du quotidien, les outils d'atelier (KKL, adaptation, commandes brutes, adresses) apparaissent en mode expert.
- 18 langues ; les réglages s'appliquent immédiatement.
- Vous prévient quand une nouvelle version est disponible : au démarrage, elle demande à GitHub la dernière version et rien d'autre (aucune donnée
  sur vous ni sur la voiture) ; peut être désactivé dans les réglages.

## Véhicules pris en charge

Tout véhicule avec OBD-II ou EOBD : en Europe les voitures à essence à partir de 2001 et les diesels à partir de 2004, aux États-Unis à partir de 1996. Les VW, Audi, Seat et Skoda plus anciennes, à partir de 1990 environ, fonctionnent avec le câble KKL. La page Véhicules indique quelle voie convient pour une voiture donnée.

| Époque | Bus | Exemples |
|---|---|---|
| jusqu'à 1995 environ | propre au constructeur | VAG avec KKL par les prises 2x2, sinon outils d'atelier uniquement |
| de 1996 à 2003 environ | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| à partir de 2004 environ | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, voitures japonaises modernes |
| États-Unis jusqu'à 2005 environ | SAE J1850 PWM et VPW | Ford, Chevrolet |
| voitures les plus récentes | OBD sur UDS (SAE J1979-2) | obligatoire aux États-Unis à partir de l'année modèle 2027, déjà présent sur certaines voitures récentes ; détecté et traduit automatiquement |
| BMW et Mini à partir de la série F | ENET (HSFZ sur Ethernet) ou D-CAN avec adressage 6F1 | tous les calculateurs |
| de nombreuses voitures à partir de 2016 environ | DoIP (ISO 13400) avec un câble DoIP | OBD sur Ethernet, trouvé automatiquement |
| Tesla Model 3 et Y | pas d'OBD : trames CAN diffusées | batterie, cellules, températures, vitesse, compteur kilométrique par écoute (câble adaptateur vers la prise de diagnostic) |

Les voitures purement électriques ne sont pas tenues de fournir les données d'émissions OBD-II. Certaines répondent en partie malgré tout ; pour les voitures qui ne répondent pas mais dont le bus est actif, Caruspex se connecte en écoute et lit ce que savent les packs intégrés (Tesla Model 3 et Y pour l'instant).

## Adaptateurs pris en charge

- Tout adaptateur compatible **ELM327** : USB, Bluetooth classique (directement par RFCOMM sous Windows et Linux une fois appairé, ou par son port COM), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE et d'autres adaptateurs avec les services GATT habituels ; trouvés par une
  recherche, sans appairage) ou Wi-Fi par TCP
- Un **câble ENET** (OBD vers RJ45) pour BMW et Mini à partir de la série F : Caruspex trouve la voiture sur le réseau
  (HSFZ, ports 6811 et 6801) et lit tous les calculateurs par Ethernet, sans puce d'adaptateur intermédiaire. L'ordinateur
  a besoin d'une adresse automatique (169.254.x.x), que Windows, macOS et Linux attribuent eux-mêmes quand aucun DHCP ne répond.
  Là aussi, Caruspex ne fait que lire ; le codage et le flashage restent à E-Sys, ISTA ou aux outils de tuning.
- Tout **câble KKL (VAG 409.1)**. Les câbles à base de FTDI ou de CP2102 sont bien plus fiables sur la K-Line que les
  clones les moins chers ; pour KWP1281, un FTDI FT232RL d'origine est recommandé.

Les interfaces propriétaires comme Delphi DS150E, Autocom ou VAS 5054A ne sont pas prises en charge, elles ne parlent pas ELM327.

| Fonction | ELM327 | KKL 409.1 |
|---|---|---|
| Lire et effacer les défauts moteur | oui | oui |
| Valeurs en direct, image figée, disponibilité | oui | oui |
| ABS, airbag, combiné d'instruments, confort | **non** | oui |
| Blocs de mesure constructeur | **non** | oui |
| **Adaptation du boîtier papillon** | **non** | oui |

C'est une limite matérielle, pas logicielle : KWP1281 est un protocole par blocs dans lequel chaque octet est
acquitté par l'autre côté. Le micrologiciel de l'ELM327 ne gère que des paires requête/réponse complètes.
Un câble KKL laisse passer la K-Line brute, si bien que Caruspex peut parler le protocole lui-même.

### Deux protocoles VAG, détectés automatiquement

| Protocole | Typique de | Réglages de base |
|---|---|---|
| KWP1281 (protocole par blocs) | moteurs 1.4 MPI | commande dédiée (bloc 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI avec Bosch MED | session de diagnostic dédiée, puis lecture de bloc |

Les canaux d'adaptation et les tests des sorties sont pour l'instant implémentés sur le chemin KWP1281 ; sur KWP2000, l'
application le dit au lieu d'échouer en silence.

Pas encore implémenté : le diagnostic constructeur sur CAN avec VW TP 2.0 (véhicules à partir de 2004 environ).

## Pile technique

| Domaine | Technologie |
|---|---|
| Langage | Rust (édition 2024) |
| Interface | Tauri 2 avec React 19 et TypeScript, construite avec Vite |
| Bibliothèques | `serialport` (ports série), `btleplug` (Bluetooth LE), `libloading` (pilotes J2534) |
| Localisation | Un seul jeu de fichiers JSON dans `translations/` pour le cœur et l'interface, 18 langues |
| Compilation | Workspace Cargo, npm pour l'interface |
| Tests | `cargo test` contre des calculateurs simulés, de fausses passerelles et un faux pilote J2534 |
| Outillage | Python 3 (générateurs de code et de tables, vérifications des traductions, simulateur ELM327) |
| Plateformes | Windows 10 et 11, Linux, macOS |

## Téléchargement

Des paquets prêts à l'emploi sont disponibles sur la [page des versions](https://github.com/SenseiIssei/Caruspex/releases) :

| Système | Fichier | Remarques |
|---|---|---|
| Windows 10 et 11, 64 bits | `Caruspex-<version>-windows-x64-setup.exe` | S'installe sans droits administrateur et peut ajouter `caruspex_cli` au chemin de recherche. Met à jour une 0.8 installée sur place et n'installe l'environnement WebView2 que s'il manque |
| Windows 10 et 11, 64 bits | `Caruspex-<version>-windows-x64.zip` | Portable, décompresser et lancer. Nécessite l'environnement WebView2, déjà présent dans Windows 10 et 11 |
| macOS 13 et plus récent, Apple Silicon | `Caruspex-<version>-macos.dmg` | Glisser l'application dans Applications |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` et lancer |
| Debian et Ubuntu, 64 bits | `Caruspex-<version>-linux-amd64.deb` | Installer avec `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Outil en ligne de commande seul | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Pour les scripts et les serveurs, sans interface |

Les paquets ne sont pas encore signés. Windows SmartScreen demande confirmation au premier lancement (« Informations complémentaires », « Exécuter quand même ») ; sur macOS,
ouvrez l'application la première fois par un clic droit et « Ouvrir ».

## Compilation

Prérequis : la chaîne d'outils Rust stable (édition 2024, Rust 1.85 ou plus récent), Node 24 avec npm, et Python 3 pour
les générateurs et les vérifications des traductions. Sous Linux, les paquets système dont l'interface et les liaisons avec l'adaptateur ont besoin
(noms Debian et Ubuntu) :

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows a besoin de l'environnement WebView2 (inclus dans Windows 10 et 11), macOS n'a besoin de rien de plus.

Installez une fois les dépendances de l'interface, puis compilez l'outil en ligne de commande et l'application :

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` lance l'application avec rechargement à chaud pendant que vous travaillez dessus. L'
interface seule, sans la fenêtre Tauri, fonctionne dans un navigateur ordinaire avec la voiture de démo intégrée :

```bash
npm --prefix app run dev
```

Ouvrez `http://localhost:5287/?demo` pour vous connecter tout de suite à la voiture de démo. `?lang=de` choisit une langue et
`?theme=light` le thème clair, par exemple `?demo&lang=de&theme=light`. Sans l'enveloppe Tauri, il n'y a pas
d'accès à l'adaptateur, la version navigateur ne connaît donc que la voiture de démo.

### Droits du port série sous Linux

Pour accéder à l'adaptateur sans `sudo` :

```bash
sudo usermod -aG dialout $USER
```

Déconnectez-vous ensuite puis reconnectez-vous.

## Produits de la compilation

| Produit | Description |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | L'application avec l'interface Tauri (`tauri build` écrit aussi les installateurs et les paquets dans `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Outil en ligne de commande scriptable sur le même cœur, voir [Outil en ligne de commande](#outil-en-ligne-de-commande) |

### Paquets et versions

`bash tools/package.sh` construit dans `dist/` les paquets du système actuel : l'installateur et deux zips sous
Windows (l'installateur nécessite [Inno Setup 6](https://jrsoftware.org/isinfo.php)), le dmg et l'archive de la ligne de commande
sous macOS, l'AppImage, le `.deb` et l'archive de la ligne de commande sous Linux. Ensuite `bash tools/smoke_test.sh` les installe ou
les ouvre comme le ferait un utilisateur, lance les deux programmes et les retire. La CI fait les deux à chaque exécution.

Une version naît d'une étiquette. `bash tools/release.sh <version>` la prépare sur sa propre branche sans rien pousser :
il fixe la version dans `Cargo.toml` et `app/package.json`, crée `changelog/<version>.md` (allemand et anglais),
lance les vérifications, les tests, les paquets et le test de fumée, et actualise les captures d'écran du README (`tools/screenshots.sh de` et
`tools/screenshots.sh en`). Une fois la pull request fusionnée, posez l'étiquette `v<version>` et poussez-la. La CI compile les
trois systèmes et publie la version avec les paquets et la liste des changements.

La CI (`.github/workflows/build.yml`) exécute d'abord le formatage, la vérification des tables générées, la vérification des traductions, la
compilation de l'interface et clippy sous Linux, puis teste, empaquette et soumet à un test de fumée Windows, Linux et macOS.
Un second workflow (`.github/workflows/data-check.yml`) cherche une fois par mois de nouveaux codes VIN, véhicules et fichiers DBC
en amont et ouvre un ticket avec les nouveautés.

## Tests

```bash
cargo test --workspace
```

Environ 370 tests s'exécutent en quelques secondes, tous sans voiture. Ils couvrent l'analyseur de réponses ELM327, le décodage
des codes défaut, la conversion des PID, les profils de véhicules et le décodeur de VIN, les services OBD-II et UDS face à la voiture de démo,
des sessions KWP1281 et KWP2000 complètes face à des calculateurs simulés, ENET (HSFZ) et DoIP face à de fausses passerelles,
SLCAN et le chemin J2534 face à un faux pilote, les enregistrements et la relecture, ainsi que l'analyse (bilan de santé, test de batterie,
ordinateur de bord, chronomètre de performances). Les tests de la ligne de commande exécutent le `caruspex_cli` compilé et comparent sa sortie avec
les fichiers de référence de `crates/caruspex-cli/tests/golden/`, qui contiennent ce que l'ancienne version C++ affichait pour les mêmes
commandes, de sorte que toute différence avec l'ancien outil apparaît.

Les vérifications que la CI exécute en plus des tests :

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

Les deux premières concernent le formatage et les avertissements du linter. `tools/i18n_rust.py --strict` échoue quand un texte du cœur manque dans
les fichiers de traduction ou qu'une traduction manque dans l'une des 18 langues ; la compilation de l'interface vérifie ses propres
textes de la même façon quand `I18N_STRICT=1` est défini. Lancez `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` et
`gen_pids.py` pour actualiser les tables générées ; la CI échoue si elles diffèrent de ce qui est versionné.

## Essayer sans voiture

Le plus rapide est la voiture de démo intégrée : choisissez « Voiture de démo » sur la page Connecter, ou lancez avec

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` liste toutes les options de l'application : `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` et `--size`.

Le simulateur Python joue soit le moteur de l'Audi A2 sur la K-Line, soit, avec `--can`, une voiture CAN avec moteur
et boîte de vitesses (en-têtes, ISO-TP, modes 06 et 09 par calculateur) :

```bash
python tools/mock_elm327.py --can
```

Connectez ensuite l'application (sous Windows, le binaire est `target/release/caruspex.exe`) :

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

L'outil en ligne de commande fonctionne aussi avec le simulateur :

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Outil en ligne de commande

`caruspex_cli <command>` avec `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` ou `--replay FILE` :

| Commande | Ce qu'elle fait |
|---|---|
| `read` | Voyant de défaut, disponibilité et codes défaut, avec la signification pour la marque quand le VIN la trahit |
| `vin` | VIN, ce qu'il dit (constructeur, pays, année modèle, chiffre de contrôle) et la voiture correspondante du catalogue |
| `freeze` | Toutes les images figées, une par défaut enregistré |
| `health` | Bilan de santé : défauts, disponibilité, mode 06 et une série en direct (`--seconds N`, 15 par défaut) |
| `probe` | Test de l'adaptateur : ce qu'est vraiment l'ELM327, quelles commandes il connaît, temps d'aller-retour |
| `uds` | UDS sur CAN : identification et mémoire de défauts de chaque calculateur (`--ids A,B,...` pour vos propres adresses) |
| `custom` | Valeurs constructeur et personnalisées d'un pack intégré (`--pack NAME`) ou d'un CSV de Torque (`--csv FILE`), une fois |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Valeurs en direct, VIN et première image figée, effacement, tout par calculateur, ports série, autotest du câble KKL |
| `enet`, `interfaces` | Cherche les BMW et Mini et les voitures DoIP sur le réseau ; liste les interfaces CAN de cet ordinateur (pilotes J2534, SocketCAN) |
| `version`, `help` | Version du programme, la liste complète des commandes et options |

`--json` affiche `read`, `vin`, `health`, `probe`, `freeze`, `uds` et `custom` en JSON pour les scripts, `--record FILE`
enregistre toute la session avec l'adaptateur pour `--replay`, `--fast-timing` met l'adaptateur en
temporisation adaptative agressive, `-v` journalise chaque commande de l'adaptateur (sur stderr avec `--json`). `caruspex_cli help` liste tout.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

Les captures d'écran de cette page viennent de la voiture de démo, prises par l'application elle-même :
`--demo --page faults --shot shot.png --size 1440x920` ouvre une page, enregistre une image de la fenêtre après quelques
secondes et quitte (`tools/screenshots.sh` prend le jeu complet).

## Structure du projet

```
crates/caruspex-i18n/       Langues : allemand et anglais dans le code, les 16 autres depuis translations/
crates/caruspex-vehicles/   Décodeur de VIN et catalogue de véhicules (marques, modèles, générations, prises, bus)
crates/caruspex-data/       Codes défaut, PID, adresses de calculateurs, correspondance code-composant
crates/caruspex-link/       Voies vers l'adaptateur (série, TCP, Bluetooth LE), pilote ELM327, contrôle de l'adaptateur
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 et KWP1281 (K-Line, octet par octet)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Enregistrements et relecture, valeurs personnalisées (CSV de Torque, packs de PID), écrivain JSON
crates/caruspex-analysis/   Bilan de santé, guide du cycle de conduite, ordinateur de bord, chronomètre de performances, test de batterie
crates/caruspex-demo/       La voiture de démo intégrée : un ELM327 avec des calculateurs simulés
crates/caruspex-cli/        L'outil en ligne de commande, avec ses tests de fichiers de référence
app/src/                    L'interface : React 19 et TypeScript (pages, structure, état, rapport, i18n)
app/src-tauri/              L'application Tauri : fenêtre, commandes, recherche de mises à jour, options de ligne de commande
translations/               Fichiers de traduction partagés pour le cœur et l'interface, 18 langues
data/                       Le catalogue de véhicules en CSV tenu à la main, sources des calculateurs et des PID
tools/                      Générateurs, vérifications des traductions, empaquetage, scripts de version et de captures, simulateur ELM327
docs/                       Captures d'écran du README
packaging/                  Script d'installateur Windows, fichier desktop Linux, Info.plist macOS
changelog/                  Notes de version par version, allemand et anglais
```

La base de codes défaut est générée : `tools/gen_dtc.py` développe les familles régulières de codes EOBD et les
listes soignées de `tools/dtc/` dans `crates/caruspex-data/src/generated/*.rs`. Relancez-le après avoir modifié les données :

```bash
python tools/gen_dtc.py
```

Le catalogue de véhicules fonctionne de la même façon : modifiez les fichiers CSV de `data/vehicles/`, puis lancez

```bash
python tools/gen_vehicles.py
```

Les adresses de calculateurs et les packs de PID ont `tools/gen_ecus.py` et `tools/gen_pids.py`. Les traductions sont décrites
dans [translations/TRANSLATING.md](translations/TRANSLATING.md) ; après l'ajout d'un texte dans le code,
`python tools/i18n_rust.py` liste ce qui reste à traduire.

## Un mot sur l'effacement des codes

Le mode 04 de l'OBD efface la mémoire de défauts, l'image figée et les moniteurs de disponibilité. Juste après, la
voiture échouera à un contrôle antipollution tant qu'elle n'aura pas accompli plusieurs cycles de conduite complets, et un code dont la cause
est toujours présente reviendra après un court trajet. Effacer n'est pas réparer.

## Avertissement

Caruspex est un projet de loisir, fourni sans garantie. Utilisez-le à vos propres risques, uniquement sur des véhicules
sur lesquels vous avez le droit d'intervenir, et jamais en conduisant. Tous les noms de produits et de marques sont des marques déposées de leurs
propriétaires respectifs ; Caruspex n'est affilié à aucun constructeur de véhicules et n'est pas approuvé par eux.

## Licence

MIT, voir [LICENSE](LICENSE). Les valeurs constructeur de centaines de modèles sont converties à partir d'[OBDb](https://github.com/OBDb) et, comme OBDb, sous licence CC BY-SA 4.0 : elles sont séparées du code dans [data/obdb](data/obdb) avec la licence et la liste des sources ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)). Les polices incluses sont sous licence SIL Open Font License 1.1 :
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) et
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
