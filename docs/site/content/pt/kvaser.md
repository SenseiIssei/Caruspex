# Kvaser (Windows)

Os adaptadores CAN da Kvaser (Leaf, U100, Memorator e outros) podem ser usados diretamente no Windows pela biblioteca CANlib da Kvaser. No Linux, eles são interfaces [SocketCAN](socketcan.md).

## Conectar

1. Instale os drivers da Kvaser. Eles trazem a `canlib32.dll`, que o Caruspex carrega.
2. Na página **Conectar**, escolha **Interface CAN**. Cada canal Kvaser com um dispositivo por trás aparece na lista; os canais virtuais do driver ficam de fora.
3. Escolha a taxa de bits (500 kbit/s para OBD via CAN) e conecte.

## O que funciona

OBD, todas as centrais via UDS, a busca paralela de centrais, VW TP 2.0 e a escuta de valores transmitidos. CAN FD via CANlib ainda não está implementado.

## Linha de comando

```
caruspex_cli read --can kvaser:0@500000
```
