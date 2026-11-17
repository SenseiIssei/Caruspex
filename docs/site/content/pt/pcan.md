# PEAK PCAN (Windows)

Os adaptadores CAN da PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro e outros) podem ser usados diretamente no Windows, sem driver J2534. No Linux, eles são interfaces [SocketCAN](socketcan.md).

## Conectar

1. Instale o driver de dispositivo da PEAK. Ele traz a biblioteca `PCANBasic.dll`, que o Caruspex carrega.
2. Na página **Conectar**, escolha **Interface CAN**. Cada canal PCAN com um dispositivo por trás aparece sozinho na lista; canais que já estão em uso por outro programa ficam de fora.
3. Escolha a taxa de bits (500 kbit/s para OBD via CAN) e conecte.

## O que funciona

OBD, todas as centrais via UDS, a busca paralela de centrais, VW TP 2.0 e a escuta de valores transmitidos. CAN FD via PCAN-Basic ainda não está implementado.

## Linha de comando

```
caruspex_cli read --can pcan:USB1@500000
```
