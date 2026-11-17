# SLCAN (Lawicel)

O SLCAN é um protocolo de texto simples, por porta serial, para pendrives CAN: CANable com o firmware slcan, USBtin, Lawicel CANUSB e muitos dispositivos caseiros. Ele funciona no Windows, Linux e macOS.

## Conectar

1. Ligue o pendrive ao computador e, com um cabo, à tomada OBD: CAN high no pino 6, CAN low no pino 14, terra nos pinos 4 e 5.
2. Na página **Conectar**, escolha **Interface CAN**. Os pendrives SLCAN são encontrados pela busca automática de portas e aparecem na lista.
3. Escolha a taxa de bits (500 kbit/s para OBD via CAN) e conecte.

## O que funciona

Como em qualquer interface CAN direta: OBD, todas as centrais via UDS, a busca paralela de centrais, VW TP 2.0 e a escuta de valores transmitidos. CAN FD não é possível via SLCAN.

## Dicas

- Um CANable com o firmware candleLight não é um dispositivo SLCAN: no Linux, ele aparece como interface [SocketCAN](socketcan.md).
- O pendrive não precisa de resistor de terminação na tomada OBD, o carro já tem o seu.

## Linha de comando

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
