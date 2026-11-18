# Vue radiographique

La **Vue radiographique** montre la voiture connectée comme un modèle en verre que vous pouvez faire tourner. À l'intérieur, vous voyez le moteur ou les moteurs électriques, la boîte de vitesses, le réservoir ou la batterie haute tension, l'échappement, le radiateur, la batterie 12 V et la prise OBD, disposés d'après le type de carrosserie et la transmission de la voiture détectée.

![Vue radiographique](screenshot-xray.png)

## Ce qui s'allume

- Les **défauts** colorent leur zone, et le calculateur responsable passe au rouge.
- Les **valeurs en direct** s'affichent en étiquettes sur la pièce à laquelle elles appartiennent : liquide de refroidissement au radiateur, régime au moteur, tension à la batterie.
- Les **calculateurs** trouvés par la recherche se placent là où ils sont habituellement, reliés par leurs lignes de données (CAN groupe motopropulseur, CAN confort, CAN diagnostic, K-Line). Pendant que Caruspex lit, de petits paquets circulent vers le calculateur interrogé.
- **Vérifier la voiture** amène la caméra sur chaque constat, l'un après l'autre.

Faites tourner avec la souris ou un doigt, zoomez avec la molette ou deux doigts ; le clavier fonctionne aussi. Les calques (carrosserie, transmission, refroidissement, carburant ou haute tension, électricité, réseau) peuvent être masqués un par un.

## Le modèle est-il exact ?

La forme vient des dimensions réelles de la génération quand elles sont connues (de Wikidata ou des données techniques du constructeur), et sinon de valeurs typiques pour le type de carrosserie et l'époque. Ce n'est pas une réplique du modèle exact : les données de conception et les schémas électriques des constructeurs ne sont pas librement disponibles. La page indique d'où vient la forme.

Si vous avez un modèle 3D de votre voiture en glTF (`.glb` ou `.gltf`), vous pouvez le charger avec **Charger votre propre modèle 3D** ; l'intérieur reste celui de la disposition.

## Sans 3D

Sur les ordinateurs sans WebGL, la page se rabat sur une vue de côté à plat avec les mêmes informations.
