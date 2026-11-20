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

## Primeiro teste no carro

A página do modo profissional **Primeiro teste no carro** guia você pela primeira sessão com um cabo KKL novo. Ela foi feita sob medida para o Audi A2 e funciona em qualquer VW, Audi, Seat ou Skoda com K-Line. Só lê: nada é apagado, nenhum teste de saídas, nenhum ajuste básico. A porta só é aberta quando um teste começa.

1. **Teste o cabo sem o carro.** Conecte o cabo só no USB, escolha a porta dele e clique em **Testar cabo**. O Caruspex lê o chip (FTDI, CH340, Prolific, CP210x), no FTDI o temporizador de latência, e mede os bits de um endereço a 5 baud na linha de break. Sem o carro nenhum eco volta, e isso está certo.
2. **Ajuste o temporizador de latência para 1 ms** (cabos FTDI no Windows): Gerenciador de Dispositivos, Portas (COM e LPT), clique duas vezes no USB Serial Port, Configurações de Porta, Avançado, Latency Timer (msec) em 1, OK, e depois desconecte e reconecte o cabo uma vez. Com os 16 ms de fábrica, a resposta aos bytes-chave pode perder a janela de 50 ms. O Caruspex lê essa configuração, mas nunca a altera.
3. **No carro:** ignição ligada, motor desligado, o cabo bem encaixado na tomada OBD e conectado direto ao computador (sem hub USB), nenhum outro programa de diagnóstico aberto. Marque a lista e clique em **Iniciar o primeiro teste**.
4. O teste desperta o motor (endereço 01) e mede cada bit do endereço, o byte de sincronização 0x55, os bytes-chave e a resposta a eles. Depois lê a identificação, a memória de falhas e os blocos de medição 001 a 003, e em seguida o painel de instrumentos 17, ABS 03, airbag 15, conforto 46 e elétrica central 09. Uma central que não responde talvez simplesmente não esteja instalada.
5. **Salvar relatório** grava um arquivo de texto e um arquivo JSON na pasta `test-reports` dentro da pasta de configurações do Caruspex. **Abrir o relatório de teste no GitHub** preenche o formulário do relatório para você. O VIN e o número do imobilizador ficam encurtados, a menos que você marque a caixa.

O que os resultados significam:

| Resultado | O que fazer |
|---|---|
| Sem eco | O cabo não ouve a si mesmo: o plugue não está encaixado, ou faltam os 12 V no pino 16 (verifique o fusível da tomada de diagnóstico). Sem o carro isso é normal. |
| O eco está distorcido | Outro dispositivo na tomada, o interruptor de um cabo K+DCAN fora da posição K-Line, ou um cabo com defeito. |
| O endereço de despertar está com o tempo irregular demais | Os bits se desviam mais de 20 ms dos 200 ms: sem hub USB, feche outros programas, temporizador de latência em 1 ms. O Caruspex tenta mais uma vez com as chamadas de break antecipadas. |
| Cabo OK, mas sem resposta do carro | A ignição está ligada? O endereço 01 é o motor. Carros a partir de 2005, mais ou menos, são diagnosticados por CAN. |
| A resposta aos bytes-chave chegou tarde demais | Ajuste o temporizador de latência para 1 ms. |

## Problemas comuns

- **A central não responde**: a ignição está ligada? O despertar a 5 baud precisa de um tempo exato; hubs USB e algumas máquinas virtuais atrapalham, conecte o cabo direto.
- **Cai depois de um tempo**: cabos baratos perdem bytes. Teste outro cabo antes de desconfiar do carro.
