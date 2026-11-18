# Cabo ENET da BMW

BMW e Mini a partir da série F (mais ou menos de 2008 em diante) têm Ethernet na tomada OBD. Um cabo ENET vai da tomada OBD até a porta de rede do computador, sem chip adaptador no meio. O Caruspex fala HSFZ com o gateway do carro (portas 6811 e 6801) e alcança todas as centrais.

## Conectar

1. Conecte o cabo no carro e na porta de rede do computador (ou num adaptador de rede USB). Ligue a ignição.
2. Espere até um minuto: sem servidor DHCP, o Windows, o macOS e o Linux dão sozinhos um endereço automático (169.254.x.x) ao computador.
3. Na página **Conectar**, escolha **Cabo de rede** e deixe o campo vazio: o Caruspex procura o carro na rede. Se a busca não encontrar nada, digite o endereço do carro.

## O que funciona

- Todas as centrais via UDS com o endereçamento da BMW, códigos de falha no formato da BMW, como o ISTA mostra (por exemplo 480A10).
- Memória de falhas, identificação e apagamento por central, valores ao vivo via OBD.
- Aqui também o Caruspex só lê. Codificação e reprogramação ficam com os programas de oficina.

## Qual cabo?

Um cabo ENET simples tem um resistor entre os pinos 8 e 16 que liga a Ethernet do carro. Cabos sem ele ficam mudos. Portas de rede de 1 Gbit/s funcionam, o carro fala a 100 Mbit/s.

## Linha de comando

```
caruspex_cli ecus --enet auto
```
