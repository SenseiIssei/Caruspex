# Simulador de tuning

La página **Simulador de tuning** (modo profesional, en Medir) es un BMW N55 virtual: el seis en línea de 3,0 litros con un turbo twin-scroll del F30 335i y del F32/F36 435i, 306 PS y 400 Nm. Cambia la presión de turbo, el encendido y la lambda por franja de régimen y mira qué dice el modelo, mucho antes de que nada toque un coche.

> **Solo simulación.** Caruspex nunca escribe mapas del motor ni reprograma. No conoce ningún formato de archivo de flasheo, ninguna suma de control ni dirección de mapa, y no copia ningún tuning comercial. El tuning real corresponde a herramientas consolidadas y a un taller, y el cambio necesita homologación (en Alemania, del TÜV). Sin ella el coche pierde su homologación.

![Simulador de tuning](screenshot-tune.png)

## Qué puedes cambiar

- Ajustes en cinco franjas de régimen (1000 a 2000, 2000 a 3500, 3500 a 5000, 5000 a 6000, 6000 a 7000): presión de turbo en bar, encendido en grados, lambda.
- Combustible (RON 95, 98, 100 o 102), temperatura exterior, presión atmosférica y el cambio (automático 8HP45 o manual de seis marchas).
- **Serie** vuelve a poner los ajustes a cero, **Restablecer** también las condiciones.

La página muestra par y potencia de serie frente a la versión cambiada, cada límite que toca el cambio con su motivo, y los valores internos: presión de turbo, aire de admisión, encendido, margen de picado, temperatura de escape, turbo, bomba de alta presión, presión media efectiva.

## Cómo calcula el modelo

1. **Aire.** El objetivo de presión queda limitado por lo que el turbo puede generar a ese régimen, por su relación de presiones y por su caudal. El compresor calienta el aire y el intercooler le quita parte del calor. Presión, temperatura y rendimiento volumétrico dan la masa de aire.
2. **Combustible y encendido.** El combustible sale del aire y la lambda. El encendido es el de serie más tu ajuste, pero nunca más adelantado de lo que permite el límite de picado: el control de picado del motor lo retiraría. El límite de picado se mueve con el octanaje, la temperatura del aire de admisión, la presión y la lambda.
3. **Potencia.** La energía del combustible por un rendimiento que baja con la distancia al mejor avance, menos el rozamiento, da par y potencia.
4. **Límites.** Turbo, temperatura de escape, bomba de alta presión, inyectores, preignición a bajo régimen, cambio, aire de admisión y mezcla dan cada uno un aviso en palabras sencillas.

Una curva entera (25 puntos de 1000 a 7000 rpm) tarda unos 0,015 ms en la versión de lanzamiento.

## Datos publicados

| Valor | Cifra | Fuente |
|---|---|---|
| Cilindrada, diámetro, carrera, compresión | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Par | 400 Nm de 1200 a 5000 rpm | Wikipedia "BMW N55", datos de BMW del 435i |
| Potencia | 225 kW (306 PS) a 5700 a 6000 rpm | Wikipedia (5700 a 5800), BMW (5800 a 6000) |
| Régimen máximo | 7000 rpm | Wikipedia "BMW N55" |
| Par de entrada nominal de la 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Suposiciones

Nadie publica con licencia libre los mapas internos del N55. Estos valores son suposiciones mías; cada una está explicada en [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- La **presión de serie** se calcula hacia atrás con el modelo a partir de la curva publicada (unos 0,7 a 0,8 bar hasta 5000 rpm, menos por encima). Que la curva de serie encaje muestra coherencia, no verdad.
- Rendimiento volumétrico 0,86 a 0,97, rendimiento indicado 0,39, rozamiento cuadrático con el régimen.
- Encendido limitado por picado de 3 a 15 grados antes del punto muerto superior con 1,5 grados de reserva; se mueve 1 grado por octano, 0,1 grado por kelvin de aire de admisión, 6 grados por bar y 2,5 grados por 0,1 de lambda.
- Turbo: relación de presiones hasta 2,55, caudal corregido hasta 0,32 kg/s, rendimiento 0,72. Eficacia del intercooler 0,85, menos con caudal alto.
- Límite de temperatura de escape 980 °C, bomba de alta presión 0,027 kg/s, ventana de inyección del 40 % del ciclo, riesgo de preignición por encima de 18 bar de presión media efectiva por debajo de 2500 rpm, manual de seis marchas unos 450 Nm.
- Combustible E5: relación aire combustible 14,4, 42,4 MJ/kg.

## ¿Qué precisión tiene?

- Serie frente a la curva publicada: 400 Nm con un 1 % de 1250 a 5000 rpm, 306 PS a 6000 rpm y 304 PS a 5750 rpm. Las pruebas admiten un 3 %.
- El modelo aún no se ha comprobado con un N55 real. Cada cifra es una estimación: aproximadamente ±8 % en par y potencia, ±0,1 bar de presión, ±10 K de aire de admisión, ±3 grados de margen de picado, ±60 K de temperatura de escape.

## Comparar con tu coche

1. Conecta, abre **Valores en vivo** con régimen, carga, presión del colector, temperatura del aire de admisión, avance del encendido, lambda y masa de aire, e inicia una grabación.
2. Acelera una vez a fondo por todo el régimen, solo donde esté permitido y sea seguro.
3. Abre la grabación en la página del simulador. Caruspex se queda con los puntos a plena carga, los agrupa en franjas de 500 rpm y muestra medido frente a simulado: presión del colector y aire de admisión con tus mapas, masa de aire y encendido con la presión y la temperatura medidas.

OBD informa la presión del colector solo hasta 255 kPa, y no está documentado qué sensor usa la centralita para la temperatura de admisión. **Ver ejemplo** llena la tabla con valores inventados para que veas cómo se lee.

## Conviene saber

- Más par que de serie a bajo régimen sube el riesgo de preignición, que puede romper pistones de un golpe.
- Un encendido más adelantado que el límite de picado no aporta nada: el control de picado lo retira.
- Un combustible mejor por sí solo da reserva, no potencia: el mapa de encendido de serie es el techo.
