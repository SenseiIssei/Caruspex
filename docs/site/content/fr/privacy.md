# Confidentialité

Caruspex fonctionne sur votre ordinateur. Il n'a ni compte, ni cloud, ni pistage.

## Ce qui est enregistré

Tout reste sur votre ordinateur :

| Quoi | Quand | Où |
|---|---|---|
| Réglages | toujours | le dossier des réglages de votre système (le bouton **Ouvrir l'emplacement de stockage** dans les réglages l'affiche) |
| Garage : VIN, voiture, contrôles, relevés de kilométrage, la dernière connexion qui a fonctionné | après la connexion d'une voiture | à côté des réglages, dans le dossier de données de Caruspex |
| Enregistrements et fichiers CSV | seulement quand vous les lancez | là où vous les enregistrez |
| Fichier journal | seulement si vous l'activez dans les réglages | le chemin que vous choisissez |
| Rapports de plantage | seulement si Caruspex plante | le dossier `crash-reports` à côté du garage |

Vous pouvez supprimer n'importe lequel de ces fichiers à tout moment.

## Ce qui est envoyé

Au démarrage, Caruspex demande à GitHub s'il existe une version plus récente. Rien sur vous ni sur la voiture n'est envoyé avec. La vérification peut être désactivée dans les réglages. En dehors de cela, Caruspex ne se connecte pas à internet. La communication avec un adaptateur Wi-Fi, un câble ENET ou une voiture DoIP reste dans votre réseau local.

## Rapports de plantage seulement sur demande

Si Caruspex plante, il écrit un court rapport sur votre ordinateur : version, système, l'endroit dans le code et la pile d'appels. Les VIN, les octets du trafic avec la voiture et votre dossier personnel en sont retirés. Rien n'est envoyé. Caruspex vous montre le rapport et propose deux boutons : **Signaler sur GitHub** ouvre dans votre navigateur un ticket prérempli que vous lisez et modifiez avant de le publier, **Ne pas signaler** le met de côté.

## Partager des enregistrements sans données personnelles

Les enregistrements de vraies voitures aident à améliorer Caruspex pour plus de voitures. **Partager l'enregistrement** prépare une copie et vous montre exactement ce qui reste et ce qui part :

- **Retiré** : les derniers chiffres du VIN (partout dans le fichier), le nom du port et l'adresse Bluetooth ou réseau de l'adaptateur, l'heure.
- **Jamais dans un enregistrement** : votre position et vos données d'utilisateur.
- **Reste** : les requêtes et les réponses, le type d'adaptateur, le protocole et la date.

Le fichier reste sur votre ordinateur jusqu'à ce que vous l'enregistriez et le joigniez vous-même à un ticket GitHub. Rien n'est envoyé automatiquement.

## Le programme de test

Le [formulaire de rapport de test](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) demande la voiture, l'année, le moteur et l'adaptateur. Il ne demande qu'un VIN raccourci ; ne publiez jamais le VIN complet, votre plaque d'immatriculation ou votre nom.
