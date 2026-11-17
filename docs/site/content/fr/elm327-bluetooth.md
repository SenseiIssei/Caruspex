# ELM327 en Bluetooth

De petits boîtiers sans câble. Il existe deux sortes de Bluetooth, et Caruspex parle les deux.

## Bluetooth LE

Les adaptateurs récents comme le Vgate iCar Pro BLE, le vLinker, l'OBDLink CX et le Veepeak BLE utilisent le Bluetooth Low Energy. Ils n'ont **pas besoin d'appairage** : sur la page **Connecter**, choisissez **Bluetooth**, cliquez sur **Rechercher des appareils Bluetooth** et choisissez l'adaptateur dans la liste. Cela fonctionne sous Windows, Linux et macOS.

## Bluetooth classique

Les adaptateurs plus anciens et beaucoup d'adaptateurs bon marché utilisent le Bluetooth classique avec un profil série.

1. Appairez l'adaptateur une fois dans les réglages Bluetooth de votre système. Le code PIN est généralement `1234` ou `0000`.
2. Dans Caruspex, choisissez **Bluetooth** et sélectionnez l'adaptateur appairé ; la liste indique si un appareil est LE, classique ou les deux.

Sous Windows et Linux, Caruspex se connecte directement aux adaptateurs classiques appairés. Sinon, Windows crée un port COM pour l'adaptateur, qui fonctionne comme l'[ELM327 en USB](elm327-usb.md). Sous macOS, les adaptateurs Bluetooth classiques ne fonctionnent que si le système leur propose un port série ; les adaptateurs Bluetooth LE sont le meilleur choix dans ce cas.

## Conseils

- Débranchez l'adaptateur quand vous ne vous en servez pas : beaucoup de modèles bon marché sollicitent la batterie de la voiture.
- Si l'adaptateur n'apparaît pas, désactivez puis réactivez le Bluetooth de l'ordinateur et vérifiez qu'aucune appli de téléphone n'y est connectée en même temps.
- Le Bluetooth est plus lent qu'un câble. Avec un OBDLink, Caruspex demande plusieurs valeurs à la fois, ce qui compense l'essentiel (voir [OBDLink et STN](obdlink-stn.md)).
