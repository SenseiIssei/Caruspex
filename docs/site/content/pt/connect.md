# Assistente de conexão e tomada OBD

## Onde fica a tomada?

Todo carro vendido na Europa desde 2001 (gasolina) ou 2004 (diesel), e nos EUA desde 1996, tem uma tomada OBD de 16 pinos ao alcance do motorista. Lugares comuns:

- embaixo do painel, à esquerda ou à direita da coluna de direção,
- acima dos pedais,
- no acabamento lateral da área dos pés,
- atrás da tampa da caixa de fusíveis,
- no console central, às vezes atrás de uma pequena tampa.

Para mais de 200 gerações de carros e 35 motos, o Caruspex sabe o lugar exato. O assistente de conexão marca esse lugar numa planta do carro e num close do lado do motorista; para motos, ele mostra uma vista lateral com o conector de diagnóstico e o cabo adaptador necessário.

> A lista de posições da tomada é uma coleção nossa e não foi conferida em todos os carros. Se o lugar estiver errado no seu, avise a gente com um [relatório de teste](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## O assistente de conexão

O assistente abre sozinho na primeira vez que você inicia o programa. Ele pergunta quatro coisas, e dá para pular qualquer passo:

1. **Qual adaptador?** ELM327 com USB, Bluetooth ou Wi-Fi, um cabo ENET ou um cabo KKL. As imagens ajudam a diferenciar.
2. **Qual carro?** Escolha no catálogo, e o assistente mostra onde fica a tomada.
3. **Ignição ligada.** A maioria das centrais só responde com a ignição ligada.
4. **Conectar.** O Caruspex procura nas portas e testa os protocolos.

Depois você encontra o assistente nas configurações e na página Conectar.

## Procurar e lembrar

Com a busca automática, o Caruspex passa por todas as portas seriais, pergunta em cada uma por um ELM327 e depois testa os protocolos do carro, um por um. Quando um carro conecta, o Caruspex guarda o caminho (adaptador, porta, protocolo, taxa de bits) junto com o VIN e tenta esse caminho primeiro da próxima vez, o que deixa a conexão mais rápida.

## Quando não conecta

Se nenhum adaptador responde ou o carro fica calado, o Caruspex mostra uma pequena lista de verificação para aquele tipo de conexão, junto com o que já descobriu: driver instalado, Bluetooth pareado, ignição ligada, taxa de bits certa, um adaptador clone que só finge ser uma versão mais nova. As páginas em [adaptadores e interfaces](interfaces.md) explicam cada tipo em detalhe.
