# Preguntas frecuentes

## ¿Caruspex cuesta algo?

No. Caruspex es software libre bajo la licencia MIT, sin anuncios, sin cuentas y sin extras de pago.

## ¿Qué adaptador compro?

Para la mayoría, lo mejor para empezar es un ELM327 USB con chip FTDI o CH340, o un OBDLink (Bluetooth o USB). Para VW, Audi, Seat y Skoda de antes de 2004 aproximadamente, añade un [cable KKL](kkl.md); para BMW y Mini a partir de la serie F, un [cable ENET](enet.md). El [resumen de interfaces](interfaces.md) los compara.

## ¿Funcionará con mi coche?

Cualquier coche con OBD-II o EOBD funciona para el motor: en Europa los de gasolina desde 2001 y los diésel desde 2004, en EE. UU. desde 1996. Los coches más antiguos del grupo VW, desde 1990 aproximadamente, funcionan con el cable KKL. La página **Vehículos** te dice qué camino funciona para un coche concreto. Las motos a partir de Euro 5 funcionan con un cable adaptador para su conector de diagnosis.

## ¿Se ha probado en mi coche?

Sinceramente: todavía no. Caruspex se prueba contra unidades de control simuladas; en hardware real solo se ha comprobado hasta ahora un OBDLink con el chip STN2232, sin coche. Ayúdanos con un [informe de prueba](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## ¿Puede Caruspex estropear mi coche?

Caruspex solo lee. Lo único que escribe es el borrado de códigos de avería, y avisa antes de hacerlo. Nunca codifica, nunca programa y nunca cambia valores de adaptación. No lo uses mientras conduces.

## ¿Puede cambiar el kilometraje, el inmovilizador o codificar funciones?

No, y no lo hará. Es algo delicado legalmente y no es el propósito de esta herramienta.

## ¿Por qué el adaptador dice versión 2.1 pero algunas cosas fallan?

Muchos adaptadores baratos dicen ser una versión de ELM327 más nueva de lo que son en realidad. La **Prueba del adaptador** averigua qué comandos conoce de verdad el adaptador.

## ¿Por qué van lentos los valores en mi coche antiguo?

Los coches con K-Line (aproximadamente de 2001 a 2007 en Europa) responden despacio por diseño. Los coches con CAN son mucho más rápidos.

## ¿Caruspex envía mis datos a algún sitio?

No. Solo la búsqueda de actualizaciones pregunta a GitHub por la versión más nueva, y se puede desactivar. Consulta [privacidad](privacy.md).

## ¿Cómo puedo ayudar?

Envía un informe de prueba de tu coche, comparte una grabación, informa de errores o mejora una traducción: la guía es [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), y el botón **Mejorar la traducción** de los ajustes la abre.

El código también es bienvenido: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) explica (en inglés) cómo compilar Caruspex y qué necesita un pull request.
