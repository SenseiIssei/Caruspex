# Cabo KKL (VAG 409.1)

Um cabo KKL é um cabo USB com um chip serial e um driver de K-Line, sem ELM327 dentro. O Caruspex controla a K-Line ele mesmo, byte por byte. É disso que VW, Audi, Seat e Skoda mais antigos precisam: o protocolo deles, o KWP1281, confirma cada byte, coisa que nenhum ELM327 consegue.

## O que funciona

- Todas as centrais de VW, Audi, Seat e Skoda de 1990 a 2004, mais ou menos: motor, câmbio, ABS, airbag, painel de instrumentos, conforto e outras.
- KWP1281 e KWP2000 (ISO 14230), detectados automaticamente pelos bytes-chave depois do despertar.
- Memória de falhas por central, blocos de medição e o ajuste básico real do corpo de borboleta (bloco de medição 060 ou 098).
- Os canais de adaptação são só leitura, de propósito. O teste de saídas aciona peças reais e só começa depois da sua confirmação.

Estas páginas estão no **modo profissional**: **Centrais via KKL**, **Corpo de borboleta** e **Adaptação e testes de saídas**. Cada uma abre o cabo por conta própria: escolha a porta serial, a central e comece.

## Qual cabo?

Cabos com um FTDI FT232RL original ou um CP2102 são bem mais confiáveis na K-Line do que os clones mais baratos. Para o KWP1281, recomenda-se um FTDI original. O cabo precisa deixar a K-Line passar direta: interfaces "HEX-USB" com firmware próprio não funcionam.

Carros de antes de 1996, mais ou menos, muitas vezes não têm tomada OBD, mas sim um conector 2x2 no cofre do motor ou embaixo do painel. Um adaptador de 2x2 para a tomada de 16 pinos é barato.

## Testar o cabo

A ferramenta de linha de comando testa o cabo sem carro:

```
caruspex_cli kklcheck --port COM3
```

## Problemas comuns

- **A central não responde**: a ignição está ligada? O despertar a 5 baud precisa de um tempo exato; hubs USB e algumas máquinas virtuais atrapalham, conecte o cabo direto.
- **Cai depois de um tempo**: cabos baratos perdem bytes. Teste outro cabo antes de desconfiar do carro.
