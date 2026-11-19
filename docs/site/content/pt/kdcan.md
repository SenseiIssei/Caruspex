# Cabo K+DCAN

O cabo K+DCAN é o cabo comum para BMW de 1998 a 2017, mais ou menos, com os programas de oficina INPA e ISTA. Dentro dele há um chip serial FTDI e um pequeno conversor para o D-CAN da BMW. Muitos desses cabos têm uma chave ou uma ponte entre os pinos 7 e 8 para carros mais antigos.

## Situação no Caruspex

> Estado real: o Caruspex lê os BMW da série E pela K-Line deste cabo: DS2 nos modelos mais antigos, KWP2000 ao estilo próprio da BMW e BMW-FAST na geração E60. Até agora só foi testado com os carros simulados do programa, ainda não num BMW real. O Caruspex ainda não fala o modo D-CAN do cabo.

## No modo K-Line

- **Interruptor em K-Line** antes de ligar o cabo à tomada; na maioria dos cabos é a posição da esquerda. Liga os pinos 7 e 8, para que também respondam as centralinas do pino 8 dos modelos mais antigos.
- **Que carros**: E36 (com a ficha redonda de 20 pinos através de um adaptador), E38, E39, E46, E52, E53, E83, E85 e a geração E60 (E60, E61, E63, E64, E65, E87, E90) até 03/2007. Depois disso, e a partir da série F, a tomada fala D-CAN ou Ethernet.
- **Como**: modo profissional ligado, página *BMW pela K-Line*, escolher a porta, *Procurar*. O Caruspex pergunta cada endereço conhecido por DS2 (9600 baud, paridade par), KWP2000 (10400 baud com inicialização rápida) e BMW-FAST (115200 baud) e mostra as centralinas encontradas com a identificação e a memória de avarias. Demora cerca de meio minuto.
- **Apagar** funciona por centralina ou em todas, sempre depois de perguntar. Depois o Caruspex volta a ler o que ficou.
- **Códigos de avaria**: a BMW não publica textos para os seus códigos, por isso o Caruspex mostra-os como número hexadecimal.
- **Só leitura**: aqui não há codificação, adaptações nem testes de atuadores; isso é trabalho do software de oficina.
- **Um cabo KKL** funciona da mesma forma, mas nos modelos mais antigos só chega às centralinas do pino 7 enquanto os pinos 7 e 8 não estiverem ligados.

Para experimentar sem carro, a lista de portas da página tem um E46 (DS2) e um E60 (KWP2000) simulados.

## O que mais funciona

- **BMW e Mini a partir da série F (mais ou menos de 2008 em diante)**: um [cabo ENET](enet.md) alcança todas as centrais via Ethernet, ou um [ELM327](elm327-usb.md) simples com o endereçamento próprio da BMW via CAN.
- **OBD padrão em BMW com CAN (mais ou menos de 2007 em diante)**: qualquer ELM327 lê falhas do motor e valores ao vivo.

## Procuramos ajuda

Se tem um cabo K+DCAN e um BMW mais antigo, um [relatório de teste](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) com o registo ajuda a confirmar a parte da K-Line num carro real.
