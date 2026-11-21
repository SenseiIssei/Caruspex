# Valores ao vivo

A página **Valores ao vivo** (modo profissional) mostra o que os sensores estão medindo agora. No modo iniciante, o **Modo direção** mostra os valores mais importantes em letras grandes.

![Valores ao vivo](screenshot-live.png)

## O que tem

- Todos os valores do modo 01 da SAE J1979 até o PID C4: 145 PIDs com mais de 300 leituras, agrupados e pesquisáveis. Quais o seu carro oferece, ele mesmo informa.
- Hodômetro, marcha, temperatura do óleo, consumo de combustível, torque, pressão do turbo, temperaturas do escapamento, filtro de partículas, AdBlue, NOx e bateria híbrida, até onde o carro informa.
- Um gráfico contínuo para vários valores ao mesmo tempo. Em janelas longas (até uma hora), o gráfico reduz os pontos antigos, mas mantém cada pico e cada vale.
- Gravação em CSV.

## Valores do fabricante e valores próprios

- **Baterias de carros elétricos**: estado de saúde, carga, tensões das células com dispersão e temperaturas para Hyundai Kona e Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf e Tesla Model 3 e Y (escutando o barramento CAN), na página **Bateria do EV**.
- **Valores OBDb** para cerca de 550 veículos a mais (pressão dos pneus, óleo, hodômetro e outros), oferecidos automaticamente para o carro detectado.
- **Sessão de diagnóstico estendida**: alguns valores dos diesel do grupo Volkswagen, como a massa de fuligem medida e a distância desde a última regeneração, a central do motor só fornece na sua sessão de diagnóstico estendida. No modo profissional o Caruspex coloca o motor nessa sessão enquanto esses valores são consultados, apenas para os ler, marca-os com **Sessão 03** e depois o devolve. Pode ser desativado nas configurações, em **Valores do fabricante**. Se a central recusar, por exemplo durante a condução, o valor mostra o motivo.
- **Seus próprios valores** no formato CSV do Torque, muito usado, com fórmulas incluídas.

![Bateria do EV](screenshot-battery.png)

## Qual a velocidade?

Quantos valores por segundo chegam depende do carro e do adaptador. Carros com CAN respondem rápido; carros na K-Line são lentos por natureza. Um OBDLink deixa o Caruspex pedir até 24 valores de uma vez (veja [OBDLink e STN](obdlink-stn.md)); uma interface CAN direta é o caminho mais rápido.

## Gravação e reprodução

Uma sessão inteira com o carro pode ser gravada num arquivo e reproduzida depois sem o carro, o que ajuda muito para achar defeitos. Com **Compartilhar gravação**, uma gravação pode ser preparada para o projeto sem dados pessoais, veja [privacidade](privacy.md).
