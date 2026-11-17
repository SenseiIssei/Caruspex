# DoIP (diagnosis por IP)

Muchos coches a partir de 2016 aproximadamente se pueden diagnosticar por Ethernet según la ISO 13400, lo que se llama DoIP. Un cable DoIP une la toma OBD con el puerto de red del ordenador.

## Conectar

1. Enchufa el cable, pon el contacto y espera a que el ordenador tenga su dirección automática (169.254.x.x), lo que puede tardar hasta un minuto.
2. En la página **Conectar** elige **Cable de red** y deja el campo vacío. Caruspex busca vehículos DoIP en la red y reconoce por sí solo si el coche habla DoIP o el HSFZ de BMW.

## Qué funciona

OBD por DoIP, y las unidades de control por UDS cuyas direcciones conoce Caruspex para la marca. Las listas de direcciones de los coches DoIP aún son cortas; las grabaciones de coches reales las van llenando (en [privacidad](privacy.md) se explica qué contiene una grabación compartida).

## ¿Qué cable?

DoIP necesita las líneas Ethernet en los pines 3, 11, 12 y 13 de la toma OBD y una línea de activación en el pin 8. Los cables que se venden para una marca concreta suelen estar cableados justo así; el cable ENET de BMW es uno de ellos.

## Línea de comandos

```
caruspex_cli enet
```

muestra los vehículos BMW y DoIP encontrados en la red.
