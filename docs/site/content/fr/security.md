# Sécurité et mises à jour

Cette page regroupe les informations de sécurité de Caruspex : à quoi sert le programme, quels risques connaître,
comment fonctionnent les mises à jour et comment le désinstaller. Elle suit les informations destinées aux
utilisateurs que demande le règlement européen sur la cyberrésilience (Cyber Resilience Act, annexe II), dans la
mesure où elles s'appliquent à un logiciel libre et non commercial.

## Le programme

- **Nom** : Caruspex, l'application `caruspex` et l'outil en ligne de commande `caruspex_cli`, pour Windows, Linux
  et macOS. La version figure dans le nom de chaque téléchargement et dans la liste des changements de chaque version.
- **Publié par** : le projet Caruspex sur [GitHub](https://github.com/SenseiIssei/Caruspex), en logiciel libre sous
  licence MIT. Questions et bogues vont dans les [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Usage prévu** : lire et expliquer les données de diagnostic des voitures, motos et camions avec votre propre
  adaptateur, sur votre propre ordinateur. Pas de compte, pas de cloud, pas de pistage ([Confidentialité](privacy.md)).
- **Propriétés de sécurité** : la lecture ne modifie jamais rien dans la voiture, et tout ce qui fait agir la
  voiture demande d'abord une confirmation. Une mise à jour n'est installée que si sa signature correspond à la clé
  intégrée à l'application. Caruspex n'ouvre aucun port réseau et n'exécute aucun code venant de fichiers ou d'internet.

## Signaler une faille {#report}

Signalez les problèmes de sécurité en privé avec le
[formulaire de nouvel avis de sécurité](https://github.com/SenseiIssei/Caruspex/security/advisories/new), pas dans
une issue publique. La [politique de sécurité](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md)
explique la suite. Les failles corrigées sont publiées comme
[avis de sécurité](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Risques à connaître {#risks}

- Téléchargez Caruspex uniquement depuis la [page des versions](https://github.com/SenseiIssei/Caruspex/releases).
  Une copie venue d'ailleurs peut avoir été modifiée.
- N'ouvrez des enregistrements, des paquets de PID et des modèles 3D que s'ils viennent de personnes de confiance.
  Ils ne peuvent pas exécuter de code, mais un fichier abîmé peut ralentir ou fermer Caruspex.
- Effacez les codes défaut, lancez les tests d'actionneurs et les réglages de base uniquement voiture à l'arrêt, dans
  un endroit sûr.
- Beaucoup d'adaptateurs Bluetooth et Wi-Fi n'ont pas de code PIN ou ont un réseau ouvert : n'importe qui à
  proximité peut s'y connecter. Débranchez l'adaptateur une fois terminé.
- Les câbles ENET, le DoIP et les adaptateurs Wi-Fi ne chiffrent rien : utilisez-les en direct ou dans votre propre
  réseau seulement.
- Les pilotes J2534 sont des programmes de leurs fabricants et tournent avec vos droits. Installez-les uniquement
  depuis le site du fabricant.

## Mises à jour et suivi {#support}

- Au démarrage, Caruspex demande à GitHub s'il existe une version plus récente et l'affiche. L'installation se fait
  en un clic, et l'application n'installe qu'un paquet dont la signature correspond. L'AppImage et l'application
  macOS se mettent à jour elles-mêmes ; le paquet .deb et l'application Android reçoivent la page de téléchargement.
- La recherche et l'installation automatique sont des interrupteurs dans les réglages, sous **Fenêtre et langue**.
  Sans la recherche, consultez la page des versions de temps en temps.
- Les correctifs de sécurité ne vont que dans la version la plus récente. Les anciennes versions ne reçoivent pas
  de correctifs séparés : gardez Caruspex à jour.
- Caruspex est entretenu sur le temps libre et n'a pas de date de fin fixe. Si l'entretien s'arrête, le dépôt et une
  dernière version l'annonceront à l'avance.

## Vérifier un téléchargement {#verify}

Chaque version contient un fichier `SHA256SUMS` avec la somme de contrôle de chaque fichier, ainsi que la preuve
signée par GitHub de la façon dont les fichiers ont été construits. Avec les fichiers dans un même dossier :

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Sans ces outils, `Get-FileHash <fichier>` dans PowerShell (Windows) ou `shasum -a 256 <fichier>` (macOS) affiche la
somme à comparer avec la ligne de `SHA256SUMS`.

## Supprimer Caruspex et vos données {#remove}

1. Désinstallez le programme : sous Windows dans Paramètres, Applications ; sous macOS en sortant Caruspex du
   dossier Applications ; sous Linux en supprimant l'AppImage ou avec `sudo apt remove caruspex`.
2. Supprimez vos données : le bouton **Ouvrir l'emplacement de stockage** des réglages montre les dossiers des
   réglages, du garage et des rapports de plantage. Supprimez-les. Les enregistrements et rapports que vous avez
   sauvegardés vous-même se trouvent là où vous les avez mis.

## Pour les développeurs et les entreprises {#integrators}

- Chaque version contient, par plateforme, une nomenclature logicielle `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX) avec chaque bibliothèque et sa version.
- L'analyse des risques, la liste des exigences et le champ d'application du Cyber Resilience Act pour Caruspex se
  trouvent dans [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex n'est pas mis sur le marché comme produit au sens du Cyber Resilience Act : il n'y a donc ni déclaration
  UE de conformité ni marquage CE.
