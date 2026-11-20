# Caruspex

O Caruspex é um programa de diagnóstico gratuito para carros e motos. Ele roda no Windows, Linux e macOS, fala 18 idiomas e funciona com os adaptadores que as pessoas realmente têm: o plugue ELM327 barato, um OBDLink, um cabo KKL para carros mais antigos do grupo Volkswagen, um cabo ENET para BMW e interfaces CAN profissionais.

![A visão geral com mostradores ao vivo](screenshot-overview.png)

## O que ele faz

- Lê e apaga códigos de falha, explica com palavras simples e diz se você pode continuar dirigindo.
- Mostra valores ao vivo: temperaturas, velocidade, mistura, pressão do turbo, tensão da bateria e muito mais, em números e em gráficos.
- Pergunta a todas as centrais do carro, não só ao motor, até onde o carro e o adaptador permitem.
- Verifica um carro usado antes da compra: o VIN confere, as quilometragens batem, as falhas foram apagadas pouco antes?
- Mostra o carro como um modelo de vidro em que falhas e valores acendem no lugar a que pertencem.
- Diz o que fazer em seguida: pare agora, leve à oficina logo, ou pode esperar.

## Por onde começar

1. [Instale o Caruspex](install.md) no seu computador.
2. Siga os [primeiros passos](first-steps.md): experimente o carro de demonstração e depois conecte o seu.
3. Procure seu adaptador em [adaptadores e interfaces](interfaces.md).
4. Leia o que o Caruspex guarda e envia em [privacidade](privacy.md). Resumindo: nada sai do seu computador a não ser que você mesmo compartilhe.

> O Caruspex quase só lê. Ele só escreve depois de perguntar a você: o apagamento de códigos de falha e, com o cabo KKL da VAG, o teste de atuadores e o ajuste básico da borboleta. Ele nunca codifica centrais, nunca muda a quilometragem e nunca mexe no imobilizador.

## Situação real

O Caruspex é feito a partir das especificações e testado com centrais simuladas, gateways falsos e drivers falsos. Em hardware real, até agora só um OBDLink com o chip STN2232 foi conferido, sem carro. Se você testar o Caruspex no seu carro, envie um [relatório de teste](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): cada relatório aumenta a lista de carros testados.
