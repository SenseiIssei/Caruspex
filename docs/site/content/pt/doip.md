# DoIP (diagnóstico via IP)

Muitos carros a partir de 2016, mais ou menos, podem ser diagnosticados via Ethernet conforme a ISO 13400, o chamado DoIP. Um cabo DoIP liga a tomada OBD à porta de rede do computador.

## Conectar

1. Conecte o cabo, ligue a ignição e espere até o computador receber o endereço automático (169.254.x.x), o que pode levar até um minuto.
2. Na página **Conectar**, escolha **Cabo de rede** e deixe o campo vazio. O Caruspex procura veículos DoIP na rede e reconhece sozinho se o carro fala DoIP ou o HSFZ da BMW.

## O que funciona

OBD via DoIP, e as centrais via UDS cujos endereços o Caruspex conhece para a marca. As listas de endereços para carros DoIP ainda são curtas; gravações de carros reais vão completando essas listas (veja em [privacidade](privacy.md) o que uma gravação compartilhada contém).

## Qual cabo?

O DoIP precisa das linhas Ethernet nos pinos 3, 11, 12 e 13 da tomada OBD e de uma linha de ativação no pino 8. Cabos vendidos para uma marca específica normalmente vêm ligados exatamente assim; o cabo ENET da BMW é um deles.

## Linha de comando

```
caruspex_cli enet
```

lista os veículos BMW e DoIP encontrados na rede.
