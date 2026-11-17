# Cabo K+DCAN

O cabo K+DCAN é o cabo comum para BMW de 1998 a 2017, mais ou menos, com os programas de oficina INPA e ISTA. Dentro dele há um chip serial FTDI e um pequeno conversor para o D-CAN da BMW. Muitos desses cabos têm uma chave ou uma ponte entre os pinos 7 e 8 para carros mais antigos.

## Situação no Caruspex

> Situação real: o Caruspex ainda não fala o modo D-CAN deste cabo. Os protocolos antigos da BMW (DS2 e KWP2000 no estilo próprio da BMW) via K-Line estão planejados, mas antes precisam de testes num carro de verdade.

O que você pode usar no lugar hoje:

- **BMW e Mini a partir da série F (mais ou menos de 2008 em diante)**: um [cabo ENET](enet.md) alcança todas as centrais via Ethernet, ou um [ELM327](elm327-usb.md) simples com o endereçamento próprio da BMW via CAN.
- **OBD padrão em BMW com CAN (mais ou menos de 2007 em diante)**: qualquer ELM327 lê falhas do motor e valores ao vivo.
- **No modo K-Line**, o cabo deixa passar a K-Line como um [cabo KKL](kkl.md). Isso serve para carros do grupo VW; centrais BMW na K-Line precisam dos protocolos que ainda faltam.

## Procuramos ajuda

Se você tem um cabo K+DCAN e uma BMW mais antiga, um [relatório de teste](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) com uma gravação ajuda a construir essa parte.
