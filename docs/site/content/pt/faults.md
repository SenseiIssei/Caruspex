# Falhas

A página **Memória de falhas** lê o que as centrais guardaram e explica com palavras simples.

![Memória de falhas](screenshot-faults.png)

## O que você vê

- Códigos **armazenados, pendentes e permanentes**. Com várias centrais, cada código mostra quem informou (motor, câmbio).
- Cerca de 3.400 códigos padrão (P, C, B, U) e os códigos de fabricante de VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai e Kia, Subaru e outros. Um código P1 é lido do jeito que a marca detectada quer dizer.
- Um **semáforo** por código e a pergunta que todo mundo faz primeiro: posso continuar dirigindo?
- As causas mais comuns, coisas que você mesmo pode verificar e uma faixa aproximada de custo.
- Onde a peça fica: o carro em vista transparente, ou a [Vista de raio X](xray.md).

## Todas as centrais

No modo profissional, a página **Todas as centrais** pergunta a todas as centrais, não só ao motor: ABS, airbag, painel de instrumentos, climatização e outras. Cada falha vem com o tipo de defeito (curto-circuito, circuito aberto, sinal implausível), número da peça e versão do software. Carros mais antigos do grupo VW são lidos com um [cabo KKL](kkl.md).

## O que devo fazer agora?

A página **O que fazer?** separa tudo o que o Caruspex encontrou em três níveis: pare agora, leve à oficina logo, pode esperar. Cada ponto tem uma frase simples explicando o porquê, uma faixa aproximada de custo e uma nota para mostrar na oficina. O OBD não consegue ver se a luz do motor está piscando ou se uma luz vermelha de aviso está acesa, então duas caixas perguntam isso a você; uma luz piscando com falhas de ignição passa o ponto para "pare agora".

![O que devo fazer agora](screenshot-todo.png)

## Apagar códigos

É possível apagar por central, com um aviso claro, e o Caruspex lê a memória de novo logo em seguida.

> Apagar não é consertar. Isso também zera os monitores de prontidão: até o carro completar vários ciclos de condução inteiros, ele reprova numa inspeção de emissões, e um código cuja causa continua lá volta depois de uma volta curta.

## Exportar

A lista pode ser salva como CSV, e o laudo de inspeção como PDF.
