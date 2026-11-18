# OBDLink e chips STN

Os adaptadores OBDLink (SX, EX, MX+, CX, LX) usam chips STN da OBD Solutions. Eles entendem tudo o que um ELM327 entende e mais; o Caruspex os reconhece sozinho e usa os extras.

## O que o Caruspex usa

- **Pedidos em lote**: o Caruspex pede até 24 valores ao vivo de uma vez, em vez de 6. Via Bluetooth, isso deixa uma rodada de valores quase duas vezes mais rápida.
- **Escuta**: valores transmitidos (por exemplo num Tesla) são gravados com os comandos próprios do STN, que param sozinhos depois de um número definido de mensagens.
- **Ford MS-CAN e GM Single-Wire-CAN**: os módulos de carroceria da Ford e da GM não ficam no CAN de diagnóstico normal. Com um adaptador que tenha o transceptor extra, a página **Todas as centrais** oferece "Ford: módulos de carroceria no MS-CAN" ou "GM: módulos de carroceria no CAN de fio único".

| Chip | Adaptador | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 a STN2232 | OBDLink EX | sim | não |
| STN1150 a 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ e outros | sim | sim |
| outros | OBDLink SX, LX, CX | não | não |

## Conectar

Os adaptadores OBDLink são conectados como qualquer ELM327: via USB (o EX e o SX), Bluetooth (MX+, LX, CX) ou como descrito em [ELM327 via USB](elm327-usb.md) e [ELM327 via Bluetooth](elm327-bluetooth.md).

## Testado

Um OBDLink com o chip STN2232 passou no teste do adaptador: chip, firmware e comandos foram reconhecidos corretamente. Ele ainda não foi usado num carro, então pedidos em lote, MS-CAN e SW-CAN ainda não estão confirmados em hardware real.
