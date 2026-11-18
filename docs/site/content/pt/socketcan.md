# SocketCAN (Linux)

O SocketCAN é a camada CAN do kernel do Linux. Qualquer adaptador CAN com driver no kernel pode ser usado por ele: PEAK PCAN-USB, Kvaser, CANable com o firmware candleLight, placas CAN para Raspberry Pi e muitos outros.

## Configurar

Ative a interface com a taxa de bits do carro (500 kbit/s para OBD via CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Para CAN FD nos carros mais novos, também com uma taxa de bits de dados:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Conectar

Na página **Conectar**, escolha **Interface CAN**. Todas as interfaces CAN do computador estão na lista (`can0`, `can1` e assim por diante). Selecione, escolha a taxa de bits e, para CAN FD, marque a caixa CAN FD.

## O que funciona

OBD, todas as centrais via UDS, a busca paralela de centrais, VW TP 2.0, a escuta de valores transmitidos e CAN FD.

## Linha de comando

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
