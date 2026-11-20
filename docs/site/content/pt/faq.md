# Perguntas frequentes

## O Caruspex custa alguma coisa?

Não. O Caruspex é software livre sob a licença MIT, sem anúncios, sem contas e sem extras pagos.

## Que adaptador devo comprar?

Para a maioria das pessoas, um ELM327 USB com chip FTDI ou CH340, ou um OBDLink (Bluetooth ou USB), é o melhor começo. Para VW, Audi, Seat e Skoda de antes de 2004, mais ou menos, acrescente um [cabo KKL](kkl.md); para BMW e Mini a partir da série F, um [cabo ENET](enet.md). A [visão geral das interfaces](interfaces.md) compara todos.

## Vai funcionar no meu carro?

Qualquer carro com OBD-II ou EOBD funciona para o motor: na Europa, carros a gasolina a partir de 2001 e diesel a partir de 2004, nos EUA a partir de 1996. Carros mais antigos do grupo VW, a partir de 1990 mais ou menos, funcionam com o cabo KKL. A página **Veículos** mostra qual caminho funciona para cada carro. Motos a partir da Euro 5 funcionam com um cabo adaptador para o conector de diagnóstico delas.

## Já foi testado no meu carro?

Sinceramente: ainda não. O Caruspex é testado com centrais simuladas; em hardware real, até agora só um OBDLink com o chip STN2232 foi conferido, sem carro. Ajude com um [relatório de teste](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## O Caruspex pode estragar meu carro?

O Caruspex quase só lê. Ele só escreve depois de perguntar a você: o apagamento de códigos de falha e, com o cabo KKL da VAG, o teste de atuadores e o ajuste básico da borboleta. Ele nunca codifica, nunca reprograma e nunca muda valores de adaptação. Não use enquanto dirige.

## Ele consegue mudar a quilometragem, o imobilizador ou codificar funções?

Não, e nunca vai. Isso é sensível do ponto de vista legal e não é o propósito desta ferramenta.

## Por que o adaptador diz versão 2.1 mas algumas coisas falham?

Muitos adaptadores baratos informam uma versão de ELM327 mais nova do que realmente são. O **Teste do adaptador** descobre quais comandos o adaptador conhece de verdade.

## Por que os valores são lentos no meu carro mais antigo?

Carros na K-Line (mais ou menos de 2001 a 2007 na Europa) respondem devagar por natureza. Carros com CAN são bem mais rápidos.

## O Caruspex envia meus dados para algum lugar?

Não. Só a verificação de atualizações pergunta ao GitHub pela versão mais nova, e apenas depois de você dizer sim; isso pode ser mudado a qualquer momento. Veja [privacidade](privacy.md).

## Como posso ajudar?

Envie um relatório de teste do seu carro, compartilhe uma gravação, relate bugs ou melhore uma tradução: o guia é o [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), e o botão **Melhorar tradução** nas configurações abre esse guia.

Código também é bem-vindo: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) explica (em inglês) como compilar o Caruspex e o que um pull request precisa.
