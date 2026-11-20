# Installation

Chaque version se trouve sur la [page des versions sur GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). Les paquets ne sont pas encore signés, Windows et macOS demandent donc une confirmation avant le premier démarrage.

## Windows 10 et 11

| Fichier | Ce que c'est |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | L'installateur. Il n'a pas besoin de droits administrateur, peut ajouter l'outil en ligne de commande `caruspex_cli` au chemin de recherche et n'installe le runtime WebView2 que s'il manque. |
| `Caruspex-<version>-windows-x64.zip` | Portable : décompressez-le et lancez `caruspex.exe`. |

Au premier démarrage, Windows SmartScreen peut indiquer que l'application est inconnue : cliquez sur "Informations complémentaires", puis sur "Exécuter quand même".

Les adaptateurs USB ont besoin d'un pilote pour leur puce USB. Windows 10 et 11 le trouvent généralement tout seuls ; sinon, voir [ELM327 en USB](elm327-usb.md).

## macOS 13 et plus récent (Apple Silicon)

Téléchargez `Caruspex-<version>-macos.dmg`, ouvrez-le et faites glisser Caruspex dans Applications. Comme l'application n'est pas encore notariée, ouvrez-la la première fois avec un clic droit et "Ouvrir".

## Linux (x86_64)

| Fichier | Ce que c'est |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Fonctionne sur presque toutes les distributions : rendez-le exécutable avec `chmod +x` et lancez-le. Il se met aussi à jour tout seul. |
| `Caruspex-<version>-linux-amd64.deb` | Pour Debian et Ubuntu : `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Pour utiliser les adaptateurs série sans `sudo`, ajoutez-vous une fois au groupe `dialout`, puis déconnectez-vous et reconnectez-vous :

```
sudo usermod -aG dialout $USER
```

## Outil en ligne de commande seul

Pour les scripts et les serveurs, il existe `caruspex_cli` sans interface, pour Windows, Linux et macOS. `caruspex_cli help` liste les commandes.

`caruspex_cli report` écrit le rapport de diagnostic dans un fichier PDF, sans l'application et par toutes les connexions que connaissent les autres commandes (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`) :

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Le rapport peut être rédigé en allemand, anglais, espagnol, français, portugais, italien, néerlandais et indonésien (`--lang`). Pour les autres langues, il sort en anglais, car les polices intégrées à tout lecteur PDF n'ont pas leurs lettres.

## Mises à jour

Au démarrage, Caruspex demande à GitHub s'il existe une version plus récente, et rien d'autre. Une nouvelle version n'est installée que si sa signature correspond à la clé intégrée à l'application. La vérification et l'installation automatique peuvent toutes deux être désactivées dans les réglages.

## Gestionnaires de paquets

Des paquets pour winget, Homebrew, Flathub et Snap sont en préparation. Tant qu'ils ne sont pas publiés, les fichiers ci-dessus sont le moyen d'installer Caruspex.
