# Simulador de tuning

A página **Simulador de tuning** (modo profissional, em Medir) é um BMW N55 virtual: o seis em linha de 3,0 litros com um turbo twin-scroll do F30 335i e do F32/F36 435i, 306 PS e 400 Nm. Altere a pressão do turbo, a ignição e o lambda por faixa de rotação e veja o que o modelo diz, muito antes de algo tocar num carro.

> **Só simulação.** O Caruspex nunca escreve mapas do motor nem reprograma. Não conhece nenhum formato de ficheiro de flash, nenhuma soma de verificação nem endereço de mapa, e não copia nenhum tuning comercial. O tuning real pertence a ferramentas estabelecidas e a uma oficina, e a alteração precisa de homologação (na Alemanha pelo TÜV). Sem ela o carro perde a homologação.

![Simulador de tuning](screenshot-tune.png)

## O que pode alterar

- Desvios em cinco faixas de rotação (1000 a 2000, 2000 a 3500, 3500 a 5000, 5000 a 6000, 6000 a 7000): pressão do turbo em bar, ignição em graus, lambda.
- Combustível (RON 95, 98, 100 ou 102), temperatura exterior, pressão atmosférica e a caixa (automática 8HP45 ou manual de seis velocidades).
- **Série** volta a pôr os desvios a zero, **Repor** também as condições.

A página mostra binário e potência de série contra a versão alterada, cada limite que a alteração toca com o motivo, e os valores internos: pressão do turbo, ar de sobrealimentação, ignição, reserva de detonação, temperatura de escape, turbo, bomba de alta pressão, pressão média efetiva.

## Como o modelo calcula

1. **Ar.** O objetivo de pressão é limitado pelo que o turbo consegue gerar a essa rotação, pela sua relação de pressão e pelo seu caudal. O compressor aquece o ar e o intercooler retira parte do calor. Pressão, temperatura e rendimento volumétrico dão a massa de ar.
2. **Combustível e ignição.** O combustível resulta do ar e do lambda. A ignição é a de série mais o seu desvio, mas nunca mais cedo do que o limite de detonação permite: o controlo de detonação do motor retirá-la-ia. O limite move-se com a octanagem, a temperatura do ar, a pressão e o lambda.
3. **Potência.** A energia do combustível vezes um rendimento que desce com a distância ao melhor avanço, menos o atrito, dá binário e potência.
4. **Limites.** Turbo, temperatura de escape, bomba de alta pressão, injetores, pré-ignição a baixa rotação, caixa, ar de sobrealimentação e mistura dão cada um um aviso em palavras simples.

Uma curva inteira (25 pontos de 1000 a 7000 rpm) demora cerca de 0,015 ms na versão de lançamento.

## Dados publicados

| Valor | Número | Fonte |
|---|---|---|
| Cilindrada, diâmetro, curso, compressão | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Binário | 400 Nm de 1200 a 5000 rpm | Wikipedia "BMW N55", dados da BMW do 435i |
| Potência | 225 kW (306 PS) a 5700 a 6000 rpm | Wikipedia (5700 a 5800), BMW (5800 a 6000) |
| Rotação máxima | 7000 rpm | Wikipedia "BMW N55" |
| Binário de entrada nominal da 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Suposições

Ninguém publica com licença livre os mapas internos do N55. Estes valores são suposições minhas; cada uma está explicada em [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- A **pressão de série** é calculada para trás com o modelo a partir da curva publicada (cerca de 0,7 a 0,8 bar até 5000 rpm, menos acima). Que a curva de série coincida mostra coerência, não verdade.
- Rendimento volumétrico 0,86 a 0,97, rendimento indicado 0,39, atrito quadrático com a rotação.
- Ignição limitada pela detonação de 3 a 15 graus antes do ponto morto superior com 1,5 graus de reserva; move-se 1 grau por octano, 0,1 grau por kelvin de ar, 6 graus por bar e 2,5 graus por 0,1 de lambda.
- Turbo: relação de pressão até 2,55, caudal corrigido até 0,32 kg/s, rendimento 0,72. Eficácia do intercooler 0,85, menos com caudal alto.
- Limite de temperatura de escape 980 °C, bomba de alta pressão 0,027 kg/s, janela de injeção de 40 % do ciclo, risco de pré-ignição acima de 18 bar de pressão média efetiva abaixo de 2500 rpm, manual de seis velocidades cerca de 450 Nm.
- Combustível E5: relação ar combustível 14,4, 42,4 MJ/kg.

## Qual a precisão?

- Série contra a curva publicada: 400 Nm com 1 % de 1250 a 5000 rpm, 306 PS a 6000 rpm e 304 PS a 5750 rpm. Os testes admitem 3 %.
- O modelo ainda não foi verificado num N55 real. Cada número é uma estimativa: cerca de ±8 % no binário e na potência, ±0,1 bar de pressão, ±10 K de ar de sobrealimentação, ±3 graus de reserva de detonação, ±60 K de temperatura de escape.

## Comparar com o seu carro

1. Ligue, abra **Valores ao vivo** com rotação, carga, pressão do coletor, temperatura do ar de admissão, avanço da ignição, lambda e massa de ar, e inicie uma gravação.
2. Acelere uma vez a fundo por toda a faixa de rotação, só onde for permitido e seguro.
3. Abra a gravação na página do simulador. O Caruspex fica com os pontos em plena carga, agrupa-os em faixas de 500 rpm e mostra medido contra simulado: pressão do coletor e ar de sobrealimentação com os seus mapas, massa de ar e ignição com a pressão e a temperatura medidas.

O OBD só informa a pressão do coletor até 255 kPa, e não está documentado que sensor a centralina usa para a temperatura de admissão. **Ver exemplo** preenche a tabela com valores inventados, para ver como se lê.

## Bom saber

- Mais binário do que a série a baixa rotação aumenta o risco de pré-ignição, que pode partir pistões de uma vez.
- Uma ignição mais adiantada do que o limite de detonação não traz nada: o controlo de detonação retira-a.
- Um combustível melhor sozinho dá reserva, não potência: o mapa de ignição de série é o teto.
