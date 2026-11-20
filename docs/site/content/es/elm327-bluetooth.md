# ELM327 por Bluetooth

Conectores pequeños sin cable. Hay dos tipos de Bluetooth, y Caruspex habla los dos.

## Bluetooth LE

Los adaptadores más nuevos, como el Vgate iCar Pro BLE, vLinker, OBDLink CX y Veepeak BLE, usan Bluetooth Low Energy. **No hace falta emparejarlos**: en la página **Conectar** elige **Bluetooth**, pulsa **Buscar dispositivos Bluetooth** y elige el adaptador de la lista. Funciona en Windows, Linux, macOS y Android.

## Bluetooth clásico

Los adaptadores más antiguos y muchos baratos usan Bluetooth clásico con perfil serie.

1. Empareja el adaptador una vez en los ajustes de Bluetooth de tu sistema. El PIN suele ser `1234` o `0000`.
2. En Caruspex elige **Bluetooth** y el adaptador emparejado; la lista indica si un dispositivo es LE, clásico o ambos.

En Windows y Linux, Caruspex se conecta directamente a los adaptadores clásicos emparejados. Como alternativa, Windows crea un puerto COM para el adaptador, que funciona como un [ELM327 por USB](elm327-usb.md). En macOS los adaptadores Bluetooth clásicos solo funcionan si el sistema les ofrece un puerto serie; ahí es mejor elegir un adaptador con Bluetooth LE.

## En Android

- Los adaptadores Bluetooth LE tampoco necesitan emparejarse aquí: elige **Bluetooth** y busca; la lista muestra la intensidad de señal de cada dispositivo.
- Los adaptadores clásicos se emparejan una vez en los ajustes de Android y después aparecen en la lista.
- Desde Android 12 el sistema pide una vez acceso a los dispositivos cercanos. Hasta Android 11, la búsqueda solo encuentra adaptadores Bluetooth LE si Caruspex puede usar la ubicación y la ubicación está activada; Caruspex no la lee. Los adaptadores clásicos emparejados funcionan sin ella.

## Consejos

- Desenchufa el adaptador cuando no lo uses: muchos de los baratos tienen la batería del coche ocupada.
- Si el adaptador no aparece, apaga y vuelve a encender el Bluetooth del ordenador y comprueba que ninguna app del móvil esté conectada a él al mismo tiempo.
- El Bluetooth es más lento que un cable. Con un OBDLink, Caruspex pide varios valores a la vez, lo que compensa la mayor parte (consulta [OBDLink y STN](obdlink-stn.md)).
