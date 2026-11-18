# Adaptadores e interfaces

O Caruspex descobre sozinho o que está conectado e o que o carro fala. Seja qual for o caminho até o carro, as mesmas funções funcionam nele: OBD-II, todas as centrais via UDS, escuta de valores transmitidos, gravação e reprodução.

| Interface | Exemplos | Página |
|---|---|---|
| ELM327 via USB | cabos ELM327 baratos, Vgate, Veepeak | [ELM327 via USB](elm327-usb.md) |
| ELM327 via Bluetooth | plugues Bluetooth clássico e Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 via Bluetooth](elm327-bluetooth.md) |
| ELM327 via Wi-Fi | plugues que abrem uma rede Wi-Fi própria | [ELM327 via Wi-Fi](elm327-wifi.md) |
| Chips STN | OBDLink SX, EX, MX+, CX | [OBDLink e STN](obdlink-stn.md) |
| Cabo KKL (VAG 409.1) | cabos FTDI ou CP2102 | [Cabo KKL](kkl.md) |
| Cabo K+DCAN | cabos BMW com chave | [Cabo K+DCAN](kdcan.md) |
| Cabo ENET | BMW e Mini a partir da série F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable com candleLight, placas CAN para Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Cabo DoIP | muitos carros a partir de 2016, mais ou menos | [DoIP](doip.md) |

## O que cada um consegue fazer?

Um ELM327 simples basta para o motor, o câmbio e, na maioria dos carros a partir de 2008, mais ou menos, todas as outras centrais via UDS. Ele não consegue o antigo protocolo KWP1281 do grupo Volkswagen, que confirma cada byte: para VW, Audi, Seat e Skoda de antes de 2004, mais ou menos, você precisa de um cabo KKL.

As interfaces CAN diretas (J2534, SLCAN, SocketCAN, PCAN, Kvaser) entregam ao Caruspex os quadros CAN puros. Ele mesmo cuida do protocolo de transporte, o que permite uma busca rápida e paralela de centrais e o VW TP 2.0 para carros do grupo Volkswagen de 2004 a 2012, mais ou menos.

![Teste do adaptador](screenshot-adapter.png)

## Meu adaptador é original?

O **Teste do adaptador** pergunta ao adaptador pelos comandos de cada versão do ELM327, mede os tempos de resposta e diz com sinceridade o que isso significa: muitos adaptadores baratos informam versão 1.5 ou 2.1, mas só conhecem os comandos da 1.0. O Caruspex nunca escreve nada no adaptador.

> Testado até agora: um OBDLink com o chip STN2232 passou no teste do adaptador, sem carro. Todos os outros adaptadores e interfaces foram feitos a partir da documentação deles, mas ainda não foram testados num carro de verdade.
