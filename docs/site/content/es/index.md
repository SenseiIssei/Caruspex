# Caruspex

Caruspex es un programa de diagnosis gratuito para coches y motos. Funciona en Windows, Linux y macOS, habla 18 idiomas y trabaja con los adaptadores que la gente tiene de verdad: el conector ELM327 barato, un OBDLink, un cable KKL para coches antiguos del grupo Volkswagen, un cable ENET para BMW e interfaces CAN profesionales.

![El resumen con indicadores en vivo](screenshot-overview.png)

## Qué hace

- Lee y borra códigos de avería, los explica con palabras sencillas y te dice si puedes seguir conduciendo.
- Muestra valores en vivo: temperaturas, velocidad, mezcla, presión de turbo, tensión de la batería y muchos más, como cifras y como gráficas.
- Pregunta a todas las unidades de control del coche, no solo al motor, hasta donde lo permitan el coche y el adaptador.
- Revisa un coche usado antes de comprarlo: ¿coincide el VIN, cuadran los kilometrajes, se borraron averías poco antes?
- Muestra el coche como un modelo de cristal en el que las averías y los valores se iluminan donde corresponden.
- Te dice qué hacer a continuación: parar ya, ir pronto al taller o puede esperar.

## Por dónde empezar

1. [Instala Caruspex](install.md) en tu ordenador.
2. Sigue los [primeros pasos](first-steps.md): prueba el coche de demostración y luego conecta el tuyo.
3. Busca tu adaptador en [adaptadores e interfaces](interfaces.md).
4. Lee qué guarda y qué envía Caruspex en [privacidad](privacy.md). En resumen: nada sale de tu ordenador a menos que lo compartas tú.

> Caruspex solo lee. Nunca codifica unidades de control, nunca cambia el kilometraje y nunca toca el inmovilizador. Borrar códigos de avería es lo único que escribe en el coche.

## Estado real

Caruspex está hecho según las especificaciones y probado contra unidades de control simuladas, gateways falsos y controladores falsos. En hardware real solo se ha comprobado hasta ahora un OBDLink con el chip STN2232, sin coche. Si pruebas Caruspex en tu coche, envía un [informe de prueba](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): cada informe alarga la lista de coches probados.
